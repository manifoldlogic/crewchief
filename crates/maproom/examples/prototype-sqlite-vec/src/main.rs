use rusqlite::{Connection, Result, ffi};
use std::os::raw::c_int;

// Declare the C extension init function
extern "C" {
    fn sqlite3_vec_init(
        db: *mut ffi::sqlite3,
        pzErrMsg: *mut *mut std::os::raw::c_char,
        pApi: *const ffi::sqlite3_api_routines,
    ) -> c_int;
}

fn main() -> Result<()> {
    println!("Opening in-memory database...");
    
    // Register the extension globally before opening connection
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }

    let conn = Connection::open_in_memory()?;

    println!("Creating vec0 table with 1536 dimensions...");
    // Create table
    conn.execute(
        "CREATE VIRTUAL TABLE vec_test USING vec0(e float[1536])",
        [],
    )?;

    println!("Inserting a 1536-dimensional vector...");
    // Create a 1536-dim vector
    let vector = vec![0.1f32; 1536];
    
    // sqlite-vec expects binary blob or JSON? documentation says float array literal or blob
    // Let's try direct parameter binding if rusqlite supports it, otherwise we might need to format it
    // sqlite-vec supports raw binary blobs for float arrays.
    // 1536 * 4 bytes = 6144 bytes.
    
    // We need to convert Vec<f32> to Vec<u8> (little endian)
    let mut blob = Vec::with_capacity(1536 * 4);
    for val in &vector {
        blob.extend_from_slice(&val.to_le_bytes());
    }

    conn.execute(
        "INSERT INTO vec_test(e) VALUES (?)",
        [&blob as &dyn rusqlite::ToSql],
    )?;

    println!("Querying for nearest neighbor...");
    // Query
    let mut stmt = conn.prepare(
        "SELECT rowid, distance FROM vec_test WHERE e MATCH ? ORDER BY distance LIMIT 1"
    )?;
    
    let rows = stmt.query_map([&blob as &dyn rusqlite::ToSql], |row| {
        let id: i64 = row.get(0)?;
        let distance: f64 = row.get(1)?;
        Ok((id, distance))
    })?;

    for row in rows {
        let (id, distance) = row?;
        println!("Found rowid: {}, distance: {}", id, distance);
        assert_eq!(id, 1);
        // distance should be ~0 for exact match
        assert!(distance < 0.00001);
    }

    println!("✅ Verification successful: sqlite-vec works with 1536 dimensions!");
    Ok(())
}

