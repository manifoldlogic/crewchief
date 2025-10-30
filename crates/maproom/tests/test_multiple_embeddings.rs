use crewchief_maproom::db::queries::{connect, upsert_embeddings};

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_store_multiple_embeddings() {
    // Connect to database
    let client = connect().await.expect("Failed to connect to database");

    // Get 10 chunks without embeddings
    let rows = client
        .query(
            "SELECT id FROM maproom.chunks
             WHERE code_embedding_ollama IS NULL
             LIMIT 10",
            &[],
        )
        .await
        .expect("Failed to query chunks");

    let chunk_ids: Vec<i64> = rows.iter().map(|row| row.get(0)).collect();

    println!("Testing embedding storage for {} chunks", chunk_ids.len());
    assert!(chunk_ids.len() >= 10, "Need at least 10 chunks for testing");

    // Create test embeddings (768 dimensions for Google/Ollama)
    let code_embedding: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
    let doc_embedding: Vec<f32> = (0..768).map(|i| (i as f32 / 768.0) * 0.5).collect();

    let mut success_count = 0;
    let mut error_count = 0;

    for chunk_id in &chunk_ids {
        let result = upsert_embeddings(
            &client,
            *chunk_id,
            Some(&code_embedding),
            Some(&doc_embedding),
            768,
        )
        .await;

        match result {
            Ok(_) => {
                success_count += 1;
                println!("✅ Stored embeddings for chunk {}", chunk_id);
            }
            Err(e) => {
                error_count += 1;
                println!("❌ Failed to store embeddings for chunk {}: {:?}", chunk_id, e);
            }
        }
    }

    println!("\n📊 Results:");
    println!("   Success: {}", success_count);
    println!("   Errors: {}", error_count);

    // Verify embeddings were stored
    let verify_row = client
        .query_one(
            "SELECT COUNT(*) as stored_count
             FROM maproom.chunks
             WHERE id = ANY($1) AND code_embedding_ollama IS NOT NULL",
            &[&chunk_ids],
        )
        .await
        .expect("Failed to verify stored embeddings");

    let stored_count: i64 = verify_row.get(0);
    println!("   Verified in database: {}", stored_count);

    // Assert at least 10 embeddings stored successfully
    assert!(
        success_count >= 10,
        "Expected at least 10 successful stores, got {}",
        success_count
    );
    assert_eq!(error_count, 0, "Expected no errors, got {}", error_count);
    assert!(
        stored_count >= 10,
        "Expected at least 10 embeddings in database, got {}",
        stored_count
    );

    println!("\n✅ SUCCESS! All {} embeddings stored without type conversion errors!", success_count);
}
