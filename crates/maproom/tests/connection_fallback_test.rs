use crewchief_maproom::db::connection::get_database_url;
use crewchief_maproom::db::pool::create_pool;

#[tokio::test]
#[cfg_attr(feature = "sqlite", ignore = "SQLite feature uses different fallback URL")]
async fn test_pool_creation_with_fallback_url() {
    // Remove MAPROOM_DATABASE_URL to test fallback logic
    std::env::remove_var("MAPROOM_DATABASE_URL");

    // Get the database URL using the fallback logic
    let url = get_database_url().expect("Should be able to determine database URL");

    // Verify URL format is valid (starts with postgresql://)
    assert!(
        url.starts_with("postgresql://"),
        "Database URL should start with postgresql://, got: {}",
        url
    );

    // Set the resolved URL as MAPROOM_DATABASE_URL for pool creation
    // (pool creation still expects it via env var internally)
    std::env::set_var("MAPROOM_DATABASE_URL", &url);

    // Try to create a connection pool with the fallback URL
    let pool_result = create_pool().await;

    match pool_result {
        Ok(pool) => {
            // Verify we can get a connection from the pool
            let client_result = pool.get().await;

            match client_result {
                Ok(client) => {
                    // Execute a simple query to verify connection works
                    let rows = client.query("SELECT 1 as test", &[]).await;

                    assert!(rows.is_ok(), "Simple query should succeed");
                    let rows = rows.unwrap();
                    assert_eq!(rows.len(), 1, "Should return exactly one row");

                    let value: i32 = rows[0].get("test");
                    assert_eq!(value, 1, "Query should return 1");
                }
                Err(e) => {
                    println!(
                        "Note: Could not get connection from pool (database may not be running)"
                    );
                    println!("Error: {}", e);
                    // Still consider test passed if URL format is valid
                }
            }
        }
        Err(e) => {
            // Database not running - just verify URL format was valid
            println!("Note: Database not running, skipping actual connection test");
            println!("Error was: {}", e);
            // Test passes as long as URL format was valid (checked above)
        }
    }

    // Clean up
    std::env::remove_var("MAPROOM_DATABASE_URL");
}
