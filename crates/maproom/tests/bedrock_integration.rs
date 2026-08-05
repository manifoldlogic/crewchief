//! End-to-end tests for the AWS Bedrock embedding provider.
//!
//! These drive [`BedrockProvider`] against a mock Bedrock Runtime endpoint, so
//! they exercise the real code path — request body construction, SigV4 signing,
//! the HTTP round trip, response parsing, batching, and error mapping — without
//! an AWS account or network access.
//!
//! The mock is reached by pointing `MAPROOM_BEDROCK_ENDPOINT_URL` at a
//! `wiremock` server, which is exactly the mechanism an enterprise uses to route
//! through a VPC endpoint or egress proxy. That makes these tests coverage for
//! the endpoint-override feature as well.
//!
//! # What is and is not covered
//!
//! Signature *correctness* is pinned by unit tests in `embedding::aws::sigv4`
//! against AWS's published vectors and an independent reference implementation.
//! These tests assert that a signature is present and well-formed on the wire;
//! only a real AWS endpoint can confirm AWS accepts it.

use std::sync::Once;

use serial_test::serial;

use maproom::embedding::bedrock::BedrockProvider;
use maproom::embedding::config::ParallelConfig;
use maproom::embedding::provider::EmbeddingProvider;
use serde_json::{json, Value};
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Install static credentials and a region once for the whole test binary.
///
/// The credential chain reads process-global environment state. Setting it once
/// up front — rather than per test — keeps these tests from racing each other,
/// and static keys mean no test can reach IMDS or STS.
static INIT_ENV: Once = Once::new();

fn init_env() {
    INIT_ENV.call_once(|| {
        std::env::set_var("AWS_ACCESS_KEY_ID", "AKIDEXAMPLE");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY");
        std::env::remove_var("AWS_SESSION_TOKEN");
        std::env::set_var("MAPROOM_BEDROCK_REGION", "us-east-1");
        // Make sure an ambient profile on the dev machine cannot influence the run.
        std::env::remove_var("AWS_PROFILE");
        std::env::remove_var("MAPROOM_AWS_PROFILE");
        std::env::set_var("AWS_EC2_METADATA_DISABLED", "true");
    });
}

/// Build a provider pointed at `endpoint`.
async fn provider_for(
    endpoint: &str,
    model: &str,
    dimension: usize,
    parallel: ParallelConfig,
) -> BedrockProvider {
    init_env();
    std::env::set_var("MAPROOM_BEDROCK_ENDPOINT_URL", endpoint);
    BedrockProvider::new(model.to_string(), dimension, parallel)
        .await
        .expect("provider construction should succeed with static credentials")
}

/// A Titan response body with `dimension` floats.
fn titan_body(dimension: usize) -> Value {
    json!({
        "embedding": vec![0.25_f32; dimension],
        "inputTextTokenCount": 7,
    })
}

/// A Cohere response body with `count` vectors of `dimension` floats.
fn cohere_body(count: usize, dimension: usize) -> Value {
    json!({ "embeddings": vec![vec![0.5_f32; dimension]; count] })
}

#[tokio::test]
#[serial]
async fn titan_v2_embeds_a_single_text_end_to_end() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(1024)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let embedding = provider.embed("fn main() {}".to_string()).await.unwrap();
    assert_eq!(embedding.len(), 1024);
    assert_eq!(provider.dimension(), 1024);
    assert_eq!(provider.provider_name(), "bedrock");
}

#[tokio::test]
#[serial]
async fn request_is_signed_and_addresses_the_encoded_model_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(1024)))
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;
    provider.embed("hello".to_string()).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let request = requests.first().expect("one request was made");

    // The ':' in the model id must be percent-encoded in the request line;
    // sending it raw produces a SignatureDoesNotMatch against real Bedrock.
    assert_eq!(
        request.url.path(),
        "/model/amazon.titan-embed-text-v2%3A0/invoke",
        "model id must be percent-encoded in the path"
    );

    let authorization = header(request, "authorization");
    assert!(
        authorization.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"),
        "unexpected Authorization header: {authorization}"
    );
    assert!(
        authorization.contains("/us-east-1/bedrock/aws4_request"),
        "credential scope must name the bedrock service and resolved region: {authorization}"
    );
    assert!(
        authorization.contains("SignedHeaders=")
            && authorization.contains("x-amz-content-sha256")
            && authorization.contains("host"),
        "host and payload hash must be signed: {authorization}"
    );
    assert!(!header(request, "x-amz-date").is_empty());
}

#[tokio::test]
#[serial]
async fn titan_v2_sends_dimensions_and_normalize() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(512)))
        .mount(&server)
        .await;

    // 512 is a documented Titan v2 Matryoshka width.
    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        512,
        ParallelConfig::bedrock_defaults(),
    )
    .await;
    let embedding = provider.embed("hello".to_string()).await.unwrap();
    assert_eq!(embedding.len(), 512);

    let requests = server.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["inputText"], "hello");
    assert_eq!(
        body["dimensions"], 512,
        "the configured width must be requested, not just validated locally"
    );
    assert_eq!(body["normalize"], true);
}

#[tokio::test]
#[serial]
async fn titan_v1_omits_v2_only_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(1536)))
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v1",
        1536,
        ParallelConfig::bedrock_defaults(),
    )
    .await;
    provider.embed("hello".to_string()).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["inputText"], "hello");
    assert!(
        body.get("dimensions").is_none(),
        "Titan v1 rejects the v2-only `dimensions` parameter"
    );
    assert!(body.get("normalize").is_none());
}

#[tokio::test]
#[serial]
async fn cohere_batches_natively_and_marks_documents() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(|request: &Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            let count = body["texts"].as_array().unwrap().len();
            ResponseTemplate::new(200).set_body_json(cohere_body(count, 1024))
        })
        // Ten texts must go out as ONE request, not ten.
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "cohere.embed-english-v3",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let texts: Vec<String> = (0..10).map(|index| format!("fn f{index}() {{}}")).collect();
    let embeddings = provider.embed_batch(texts).await.unwrap();
    assert_eq!(embeddings.len(), 10);
    assert!(embeddings.iter().all(|vector| vector.len() == 1024));

    let requests = server.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["texts"].as_array().unwrap().len(), 10);
    assert_eq!(body["input_type"], "search_document");
    assert_eq!(body["truncate"], "END");
}

#[tokio::test]
#[serial]
async fn query_handle_switches_cohere_input_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(cohere_body(1, 1024)))
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "cohere.embed-multilingual-v3",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;
    provider
        .for_queries()
        .embed("where is auth handled".to_string())
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        body["input_type"], "search_query",
        "queries must be encoded as queries or retrieval quality drops"
    );
}

#[tokio::test]
#[serial]
async fn titan_batch_fans_out_to_one_request_per_text_in_order() {
    let server = MockServer::start().await;
    // Echo the input's index back as the vector's first element so ordering is
    // verifiable even though requests complete concurrently and out of order.
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(|request: &Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            let text = body["inputText"].as_str().unwrap();
            let index: f32 = text.parse().unwrap();
            let mut embedding = vec![0.0_f32; 1024];
            embedding[0] = index;
            ResponseTemplate::new(200)
                .set_body_json(json!({ "embedding": embedding, "inputTextTokenCount": 1 }))
        })
        .expect(25)
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let texts: Vec<String> = (0..25).map(|index| index.to_string()).collect();
    let embeddings = provider.embed_batch(texts).await.unwrap();

    assert_eq!(embeddings.len(), 25);
    for (index, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding[0], index as f32,
            "parallel sub-batches must be reassembled in input order"
        );
    }
}

#[tokio::test]
#[serial]
async fn oversized_cohere_batch_splits_at_the_api_limit() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(|request: &Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            let texts = body["texts"].as_array().unwrap();
            assert!(
                texts.len() <= 96,
                "Bedrock rejects Cohere batches above 96; got {}",
                texts.len()
            );
            ResponseTemplate::new(200).set_body_json(cohere_body(texts.len(), 1024))
        })
        // 200 texts / 96 per request = 3 requests.
        .expect(3)
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "cohere.embed-english-v3",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let texts: Vec<String> = (0..200).map(|index| format!("text {index}")).collect();
    let embeddings = provider.embed_batch(texts).await.unwrap();
    assert_eq!(embeddings.len(), 200);
}

#[tokio::test]
#[serial]
async fn throttling_is_retried_and_then_succeeds() {
    let server = MockServer::start().await;

    // First attempt throttles, second succeeds. Scoped mocks are matched in
    // registration order, so this reliably models a transient 429.
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-amzn-errortype", "ThrottlingException:http://internal")
                .set_body_json(json!({ "message": "Too many requests" })),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(1024)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let embedding = provider.embed("retry me".to_string()).await.unwrap();
    assert_eq!(embedding.len(), 1024, "a 429 must be retried, not surfaced");
}

#[tokio::test]
#[serial]
async fn access_denied_is_not_retried_and_explains_model_access() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("x-amzn-errortype", "AccessDeniedException")
                .set_body_json(json!({ "message": "not authorized to invoke this model" })),
        )
        // Exactly one attempt: retrying an authorization failure only wastes time.
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let error = provider.embed("nope".to_string()).await.unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("bedrock:InvokeModel"),
        "the error should name the missing IAM action: {message}"
    );
    assert!(
        message.contains("Model access"),
        "the error should mention Bedrock model access enablement: {message}"
    );
}

#[tokio::test]
#[serial]
async fn unknown_model_error_names_the_region() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("x-amzn-errortype", "ResourceNotFoundException")
                .set_body_json(json!({ "message": "model not found" })),
        )
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "cohere.embed-english-v3",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let message = provider.embed("x".to_string()).await.unwrap_err().to_string();
    assert!(
        message.contains("us-east-1"),
        "region matters — model availability differs per region: {message}"
    );
}

#[tokio::test]
#[serial]
async fn dimension_mismatch_is_reported_rather_than_stored() {
    let server = MockServer::start().await;
    // Configured for 1024 but the endpoint returns 768. Storing this would
    // corrupt the index; it must fail loudly.
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(768)))
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let message = provider.embed("x".to_string()).await.unwrap_err().to_string();
    assert!(message.contains("1024") && message.contains("768"), "{message}");
}

#[tokio::test]
#[serial]
async fn cost_and_request_metrics_are_tracked() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(1024)))
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    for _ in 0..3 {
        provider.embed("x".to_string()).await.unwrap();
    }

    let metrics = provider.metrics().expect("Bedrock tracks metrics");
    assert_eq!(metrics.total_requests, 3);
    assert_eq!(metrics.failed_requests, 0);
    // Titan reports inputTextTokenCount = 7 per call.
    assert_eq!(metrics.total_tokens, 21);
    assert!(
        metrics.estimated_cost_usd > 0.0,
        "token-priced models must produce a non-zero estimate"
    );
}

#[tokio::test]
#[serial]
async fn empty_batch_makes_no_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(1024)))
        .expect(0)
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    assert!(provider.embed_batch(Vec::new()).await.unwrap().is_empty());
}

#[tokio::test]
#[serial]
async fn cohere_response_keyed_by_embedding_type_is_accepted() {
    let server = MockServer::start().await;
    // Cohere returns `{"embeddings": {"float": [...]}}` when embedding_types is
    // negotiated. Accepting both shapes keeps a future API change from silently
    // yielding zero embeddings.
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "embeddings": { "float": vec![vec![0.1_f32; 1024]] }
        })))
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "cohere.embed-english-v3",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    let embedding = provider.embed("x".to_string()).await.unwrap();
    assert_eq!(embedding.len(), 1024);
}

#[tokio::test]
#[serial]
async fn session_token_credentials_add_the_security_token_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/model/.*/invoke$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(titan_body(1024)))
        .mount(&server)
        .await;

    init_env();
    std::env::set_var("AWS_SESSION_TOKEN", "temporary-session-token");
    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;
    provider.embed("x".to_string()).await.unwrap();
    std::env::remove_var("AWS_SESSION_TOKEN");

    let requests = server.received_requests().await.unwrap();
    let request = requests.last().unwrap();
    assert_eq!(
        header(request, "x-amz-security-token"),
        "temporary-session-token"
    );
    assert!(
        header(request, "authorization").contains("x-amz-security-token"),
        "the session token must be inside SignedHeaders"
    );
}

#[tokio::test]
#[serial]
async fn empty_text_is_rejected_before_any_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(""))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = provider_for(
        &server.uri(),
        "amazon.titan-embed-text-v2:0",
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await;

    assert!(provider.embed("   ".to_string()).await.is_err());
}

#[tokio::test]
#[serial]
async fn unsupported_dimension_fails_at_construction() {
    init_env();
    // 768 is not one of Titan v2's Matryoshka widths. Catching this up front
    // beats discovering it after a multi-hour scan produced an unusable index.
    let error = BedrockProvider::new(
        "amazon.titan-embed-text-v2:0".to_string(),
        768,
        ParallelConfig::bedrock_defaults(),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("768"));
}

#[tokio::test]
#[serial]
async fn unknown_model_fails_at_construction_with_guidance() {
    init_env();
    let error = BedrockProvider::new(
        "meta.llama-embed-v9".to_string(),
        1024,
        ParallelConfig::bedrock_defaults(),
    )
    .await
    .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("amazon.titan-embed-text-v2:0"), "{message}");
    assert!(message.contains("MAPROOM_EMBEDDING_DIMENSION"), "{message}");
}

/// Read a header value from a recorded request.
fn header(request: &Request, name: &str) -> String {
    request
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}
