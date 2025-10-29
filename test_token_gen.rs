use google_cloud_auth::token::DefaultTokenSourceProvider;
use google_cloud_token::TokenSourceProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set credentials path
    std::env::set_var(
        "GOOGLE_APPLICATION_CREDENTIALS",
        "/home/vscode/.config/gcp/maproom-sa-key.json",
    );

    // Create token source provider
    let ts_provider = DefaultTokenSourceProvider::new(google_cloud_auth::project::Config {
        audience: None,
        scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
        sub: None,
    })
    .await?;

    // Get token
    let token_source = ts_provider.token_source();
    let token = token_source.token().await?;

    println!("Token obtained successfully!");
    println!("Token length: {}", token.len());
    println!("Token prefix: {}...", &token[..std::cmp::min(50, token.len())]);

    // Test the actual API call with this token
    let client = reqwest::Client::new();
    let project_id = "crewchief-476600";
    let region = "us-central1";
    let model = "textembedding-gecko@003";

    let url = format!(
        "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predict",
        region, project_id, region, model
    );

    println!("\nTesting API call to: {}", url);

    let request_body = serde_json::json!({
        "instances": [{
            "content": "test text",
            "task_type": "RETRIEVAL_DOCUMENT"
        }]
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    println!("Response status: {}", status);

    let response_text = response.text().await?;
    println!("Response body: {}", response_text);

    Ok(())
}
