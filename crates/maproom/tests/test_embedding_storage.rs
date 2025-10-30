use crewchief_maproom::db::queries::{connect, upsert_embeddings};

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_embedding_storage_with_pgvector() {
    // Connect to database
    let client = connect().await.expect("Failed to connect to database");

    // Create test embeddings (768 dimensions for Google/Ollama)
    let code_embedding: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
    let text_embedding: Vec<f32> = (0..768).map(|i| (i as f32 / 768.0) * 0.5).collect();

    // Test chunk ID (from database query above)
    let chunk_id = 209116i64;

    println!("Testing embedding storage for chunk {}", chunk_id);
    println!("Code embedding length: {}", code_embedding.len());
    println!("Text embedding length: {}", text_embedding.len());

    // Try to upsert embeddings - this should work with pgvector::Vector conversion
    let result = upsert_embeddings(
        &client,
        chunk_id,
        Some(&code_embedding),
        Some(&text_embedding),
        768,
    )
    .await;

    match result {
        Ok(_) => {
            println!("✅ SUCCESS! Embeddings stored successfully for chunk {}", chunk_id);

            // Verify embeddings were actually stored
            let row = client
                .query_one(
                    "SELECT vector_dims(code_embedding_ollama) as code_dim,
                            vector_dims(text_embedding_ollama) as text_dim
                     FROM maproom.chunks WHERE id = $1",
                    &[&chunk_id],
                )
                .await
                .expect("Failed to query stored embeddings");

            let code_dim: Option<i32> = row.get(0);
            let text_dim: Option<i32> = row.get(1);

            println!("✅ Verified: code_dim={:?}, text_dim={:?}", code_dim, text_dim);
            assert_eq!(code_dim, Some(768));
            assert_eq!(text_dim, Some(768));
        }
        Err(e) => {
            panic!("❌ FAILED! Error storing embeddings: {:?}", e);
        }
    }
}
