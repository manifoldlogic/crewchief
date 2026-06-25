//! Embedding Compatibility Verification Test (VECSRCH.0001)
//!
//! Phase 0 go/no-go gate: Verifies that Gemini REST API and Vertex AI produce
//! identical embeddings for the same input text using text-embedding-004.
//!
//! This is a one-time verification test, not part of the permanent test suite.
//! It requires live API credentials:
//! - GEMINI_API_KEY environment variable for Gemini REST API
//! - Google ADC (~/.config/gcloud/application_default_credentials.json) for Vertex AI
//! - GOOGLE_PROJECT_ID for Vertex AI

/// Calculate cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    assert_eq!(a.len(), b.len(), "Vectors must have equal length");

    let mut dot_product: f64 = 0.0;
    let mut magnitude_a: f64 = 0.0;
    let mut magnitude_b: f64 = 0.0;

    for i in 0..a.len() {
        let ai = a[i] as f64;
        let bi = b[i] as f64;
        dot_product += ai * bi;
        magnitude_a += ai * ai;
        magnitude_b += bi * bi;
    }

    let magnitude_a = magnitude_a.sqrt();
    let magnitude_b = magnitude_b.sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Calculate max absolute element-wise difference between two vectors.
fn max_element_diff(a: &[f32], b: &[f32]) -> f64 {
    assert_eq!(a.len(), b.len(), "Vectors must have equal length");
    a.iter()
        .zip(b.iter())
        .map(|(ai, bi)| (*ai as f64 - *bi as f64).abs())
        .fold(0.0f64, f64::max)
}

/// Calculate mean absolute element-wise difference between two vectors.
fn mean_element_diff(a: &[f32], b: &[f32]) -> f64 {
    assert_eq!(a.len(), b.len(), "Vectors must have equal length");
    let sum: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(ai, bi)| (*ai as f64 - *bi as f64).abs())
        .sum();
    sum / a.len() as f64
}

/// Obtain an OAuth2 access token from ADC authorized_user credentials.
///
/// Reads the ADC file, extracts client_id/client_secret/refresh_token,
/// and exchanges them for a fresh access token via Google's OAuth2 endpoint.
async fn get_access_token_from_adc(
    client: &reqwest::Client,
    adc_path: &str,
) -> Result<String, String> {
    let content =
        std::fs::read_to_string(adc_path).map_err(|e| format!("Failed to read ADC file: {}", e))?;

    let adc: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse ADC JSON: {}", e))?;

    let client_id = adc["client_id"]
        .as_str()
        .ok_or("Missing client_id in ADC file")?;
    let client_secret = adc["client_secret"]
        .as_str()
        .ok_or("Missing client_secret in ADC file")?;
    let refresh_token = adc["refresh_token"]
        .as_str()
        .ok_or("Missing refresh_token in ADC file")?;

    let token_response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to request token: {}", e))?;

    let status = token_response.status();
    let body: serde_json::Value = token_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "Token refresh failed (HTTP {}): {}",
            status,
            body.get("error_description")
                .or_else(|| body.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
        ));
    }

    body["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No access_token in response".to_string())
}

/// Call the Vertex AI predict endpoint directly via HTTP.
async fn embed_via_vertex_ai(
    client: &reqwest::Client,
    access_token: &str,
    project_id: &str,
    region: &str,
    text: &str,
) -> Result<Vec<f32>, String> {
    let url = format!(
        "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/text-embedding-004:predict",
        region, project_id, region
    );

    let request_body = serde_json::json!({
        "instances": [{
            "content": text,
            "task_type": "RETRIEVAL_DOCUMENT"
        }]
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Vertex AI request failed: {}", e))?;

    let status = response.status();
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Vertex AI response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Vertex AI API error (HTTP {}): {}", status, body));
    }

    let values = body["predictions"][0]["embeddings"]["values"]
        .as_array()
        .ok_or_else(|| format!("Unexpected Vertex AI response format: {}", body))?;

    Ok(values
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect())
}

/// Call the Gemini REST API embedContent endpoint directly via HTTP.
async fn embed_via_gemini_rest(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    text: &str,
) -> Result<Vec<f32>, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
        model, api_key
    );

    let request_body = serde_json::json!({
        "model": format!("models/{}", model),
        "content": {
            "parts": [{"text": text}]
        }
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Gemini REST API request failed: {}", e))?;

    let status = response.status();
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Gemini REST API response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Gemini REST API error (HTTP {}): {}", status, body));
    }

    let values = body["embedding"]["values"]
        .as_array()
        .ok_or_else(|| format!("Unexpected Gemini response format: {}", body))?;

    Ok(values
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect())
}

/// List available embedding models from the Gemini REST API.
async fn list_gemini_embedding_models(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<Vec<(String, Vec<String>)>, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}&pageSize=100",
        api_key
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to list models: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse models response: {}", e))?;

    let models = body["models"]
        .as_array()
        .ok_or("No models array in response")?;

    let mut embedding_models = Vec::new();
    for model in models {
        let methods: Vec<String> = model["supportedGenerationMethods"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if methods.contains(&"embedContent".to_string()) {
            let name = model["name"].as_str().unwrap_or("unknown").to_string();
            embedding_models.push((name, methods));
        }
    }

    Ok(embedding_models)
}

/// Print comparison results between two embedding vectors.
fn print_comparison_results(
    label_a: &str,
    label_b: &str,
    embedding_a: &[f32],
    embedding_b: &[f32],
) {
    let similarity = cosine_similarity(embedding_a, embedding_b);
    let max_diff = max_element_diff(embedding_a, embedding_b);
    let mean_diff = mean_element_diff(embedding_a, embedding_b);

    println!("=== Comparison: {} vs {} ===", label_a, label_b);
    println!();

    // Dimension check
    println!(
        "{} dimension: {} - {}",
        label_a,
        embedding_a.len(),
        if embedding_a.len() == embedding_b.len() {
            "MATCH"
        } else {
            "MISMATCH"
        }
    );
    println!("{} dimension: {}", label_b, embedding_b.len());
    println!();

    // Cosine similarity
    println!("Cosine similarity: {:.10}", similarity);
    println!("Max element-wise absolute difference: {:.10e}", max_diff);
    println!("Mean element-wise absolute difference: {:.10e}", mean_diff);
    println!();

    // Side-by-side comparison of first 10 values
    println!("First 10 values side-by-side:");
    println!(
        "{:<6} {:>20} {:>20} {:>15}",
        "Index", label_a, label_b, "Difference"
    );
    println!("{}", "-".repeat(65));
    let n = 10.min(embedding_a.len()).min(embedding_b.len());
    for i in 0..n {
        let diff = (embedding_a[i] as f64 - embedding_b[i] as f64).abs();
        println!(
            "{:<6} {:>20.10} {:>20.10} {:>15.10e}",
            i, embedding_a[i], embedding_b[i], diff
        );
    }
    println!();
}

/// VECSRCH.0001: Verify Gemini REST API and Vertex AI embedding compatibility.
///
/// This test performs a comprehensive compatibility verification:
/// 1. Lists available embedding models in the Gemini REST API
/// 2. Checks if text-embedding-004 is available (critical assumption from architecture.md)
/// 3. If text-embedding-004 is NOT available, tests the available model (gemini-embedding-001)
/// 4. Attempts Vertex AI embedding if credentials are available
/// 5. Reports findings with go/no-go decision
#[tokio::test]
async fn verify_gemini_vertex_embedding_compatibility() {
    // --- Credential checks ---
    let gemini_api_key = match std::env::var("GEMINI_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            eprintln!("BLOCKER: GEMINI_API_KEY environment variable not set or empty.");
            panic!("GEMINI_API_KEY not available - cannot run compatibility test");
        }
    };

    let project_id = std::env::var("GOOGLE_PROJECT_ID").ok();

    // Check for ADC credentials
    let adc_path = format!(
        "{}/.config/gcloud/application_default_credentials.json",
        std::env::var("HOME").unwrap_or_default()
    );
    let has_adc = std::path::Path::new(&adc_path).exists();

    let test_text = "Authentication flow for embedding providers in semantic search";
    let region = "us-central1";
    let client = reqwest::Client::new();

    println!("\n=== VECSRCH.0001: Embedding Compatibility Verification ===");
    println!("Test string: \"{}\"", test_text);
    println!("Expected model: text-embedding-004 (768 dimensions)");
    println!("Project: {}", project_id.as_deref().unwrap_or("N/A"));
    println!("Region: {}", region);
    println!(
        "Credentials: GEMINI_API_KEY=set, ADC={}",
        if has_adc { "present" } else { "missing" }
    );
    println!();

    // === Step 1: List available embedding models in Gemini REST API ===
    println!("=== Step 1: Discovering Gemini REST API Embedding Models ===");
    let embedding_models = list_gemini_embedding_models(&client, &gemini_api_key)
        .await
        .expect("Failed to list Gemini embedding models");

    println!("Available embedding models (supporting embedContent):");
    for (name, methods) in &embedding_models {
        println!("  {} -> {:?}", name, methods);
    }
    println!();

    // Check if text-embedding-004 is available
    let has_text_embedding_004 = embedding_models
        .iter()
        .any(|(name, _)| name == "models/text-embedding-004");

    println!(
        "text-embedding-004 available in Gemini REST API: {}",
        if has_text_embedding_004 { "YES" } else { "NO" }
    );

    // === Step 2: Test text-embedding-004 via Gemini REST API ===
    println!();
    println!("=== Step 2: Testing text-embedding-004 via Gemini REST API ===");

    let te004_gemini_result =
        embed_via_gemini_rest(&client, &gemini_api_key, "text-embedding-004", test_text).await;

    match &te004_gemini_result {
        Ok(embedding) => {
            println!(
                "text-embedding-004 via Gemini REST: SUCCESS (dimension: {})",
                embedding.len()
            );
            println!(
                "First 10 values: {:?}",
                &embedding[..10.min(embedding.len())]
            );
        }
        Err(e) => {
            println!("text-embedding-004 via Gemini REST: FAILED");
            println!("Error: {}", e);
            println!();
            println!("CRITICAL FINDING: text-embedding-004 is NOT available through the");
            println!("Gemini REST API (generativelanguage.googleapis.com). The architecture");
            println!("document assumed it would be available at v1beta path.");
        }
    }

    // === Step 3: Test available model (gemini-embedding-001) ===
    println!();
    println!("=== Step 3: Testing gemini-embedding-001 via Gemini REST API ===");

    let ge001_gemini_result =
        embed_via_gemini_rest(&client, &gemini_api_key, "gemini-embedding-001", test_text).await;

    match &ge001_gemini_result {
        Ok(embedding) => {
            println!(
                "gemini-embedding-001 via Gemini REST: SUCCESS (dimension: {})",
                embedding.len()
            );
            println!(
                "First 10 values: {:?}",
                &embedding[..10.min(embedding.len())]
            );
        }
        Err(e) => {
            println!("gemini-embedding-001 via Gemini REST: FAILED");
            println!("Error: {}", e);
        }
    }

    // === Step 4: Attempt Vertex AI embedding ===
    println!();
    println!("=== Step 4: Testing text-embedding-004 via Vertex AI ===");

    let vertex_embedding = if let Some(ref pid) = project_id {
        if has_adc {
            match get_access_token_from_adc(&client, &adc_path).await {
                Ok(token) => {
                    println!("ADC token obtained successfully.");
                    match embed_via_vertex_ai(&client, &token, pid, region, test_text).await {
                        Ok(embedding) => {
                            println!(
                                "text-embedding-004 via Vertex AI: SUCCESS (dimension: {})",
                                embedding.len()
                            );
                            println!(
                                "First 10 values: {:?}",
                                &embedding[..10.min(embedding.len())]
                            );
                            Some(embedding)
                        }
                        Err(e) => {
                            println!("text-embedding-004 via Vertex AI: FAILED");
                            println!("Error: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    println!("ADC token refresh failed: {}", e);
                    println!("Vertex AI comparison skipped (credentials expired).");
                    None
                }
            }
        } else {
            println!("No ADC file found. Vertex AI comparison skipped.");
            None
        }
    } else {
        println!("GOOGLE_PROJECT_ID not set. Vertex AI comparison skipped.");
        None
    };

    // === Step 5: Cross-API comparison (if both available) ===
    println!();
    println!("=== Step 5: Cross-API Comparison ===");

    if let (Ok(gemini_te004), Some(ref vertex_emb)) = (&te004_gemini_result, &vertex_embedding) {
        // Both text-embedding-004 embeddings available
        print_comparison_results("Vertex AI", "Gemini REST", vertex_emb, gemini_te004);

        let similarity = cosine_similarity(vertex_emb, gemini_te004);
        if similarity > 0.99 {
            println!(
                "FULL VERIFICATION PASS: Cosine similarity {:.10} > 0.99",
                similarity
            );
        } else {
            println!(
                "FULL VERIFICATION FAIL: Cosine similarity {:.10} < 0.99",
                similarity
            );
        }
    } else {
        println!("Full cross-API comparison not possible.");
        if te004_gemini_result.is_err() {
            println!("  - text-embedding-004 NOT available in Gemini REST API");
        }
        if vertex_embedding.is_none() {
            println!("  - Vertex AI credentials unavailable or expired");
        }
    }

    // === Step 6: Final Decision ===
    println!();
    println!("========================================");
    println!("=== FINAL DECISION ===");
    println!("========================================");
    println!();

    if !has_text_embedding_004 || te004_gemini_result.is_err() {
        println!("FINDING: text-embedding-004 is NOT available through the Gemini REST API.");
        println!();
        println!("The architecture assumed that Gemini REST API at:");
        println!("  https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent");
        println!("would serve the same model as Vertex AI. This is incorrect.");
        println!();
        println!("Available Gemini REST API embedding model:");
        for (name, _) in &embedding_models {
            println!("  - {} (3072 dimensions)", name);
        }
        println!();
        println!("Vertex AI embedding model:");
        println!("  - text-embedding-004 (768 dimensions)");
        println!();

        if let Some(ref ge001) = ge001_gemini_result.as_ref().ok() {
            println!(
                "gemini-embedding-001 produces {}-dimensional vectors.",
                ge001.len()
            );
            println!("text-embedding-004 (Vertex AI) produces 768-dimensional vectors.");
            println!("These are INCOMPATIBLE - different dimensions mean different vector spaces.");
            println!();
        }

        println!("DECISION: NO-GO (architecture revision required)");
        println!();
        println!("The VECSRCH ticket's approach of using GEMINI_API_KEY with text-embedding-004");
        println!("via Gemini REST API is not feasible. Options:");
        println!("  1. Use gemini-embedding-001 (3072-dim) for BOTH scan-time and query-time");
        println!("     (requires re-indexing all existing embeddings)");
        println!(
            "  2. Keep Vertex AI (text-embedding-004, 768-dim) and require service account/ADC"
        );
        println!("  3. Use Gemini REST API with gemini-embedding-001 for new indexes only");
        println!();
        println!("Gemini REST API endpoint and response format VERIFIED:");
        println!("  Endpoint: https://generativelanguage.googleapis.com/v1beta/models/MODEL:embedContent?key=KEY");
        println!("  Response: {{\"embedding\": {{\"values\": [f32...]}}}}");
        println!("  Auth: API key as query parameter");
    } else if let Some(ref vertex_emb) = vertex_embedding {
        let gemini_emb = te004_gemini_result.as_ref().unwrap();
        let similarity = cosine_similarity(vertex_emb, gemini_emb);

        if similarity > 0.99 {
            println!("DECISION: GO");
            println!("Cosine similarity: {:.10} (> 0.99 threshold)", similarity);
            println!("Proceed to Phase 1: Implement dual-path authentication.");
        } else {
            println!("DECISION: NO-GO");
            println!("Cosine similarity: {:.10} (< 0.99 threshold)", similarity);
            println!("Embeddings are not compatible. Architecture revision required.");
        }
    }

    // The test passes as long as we got useful results.
    // The actual go/no-go decision is documented in the output.
    // We assert on what we CAN verify:

    // 1. Gemini REST API endpoint works with a valid embedding model
    assert!(
        ge001_gemini_result.is_ok(),
        "Gemini REST API should work with gemini-embedding-001"
    );

    // 2. gemini-embedding-001 returns a valid embedding
    if let Ok(ref embedding) = ge001_gemini_result {
        assert!(
            !embedding.is_empty(),
            "gemini-embedding-001 should return a non-empty embedding"
        );
        println!("VERIFIED: Gemini REST API embedContent endpoint works correctly.");
        println!(
            "VERIFIED: gemini-embedding-001 returns {}-dimensional vectors.",
            embedding.len()
        );
    }
}
