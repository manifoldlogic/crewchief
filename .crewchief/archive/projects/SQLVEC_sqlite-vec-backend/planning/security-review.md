# Security Review: SQLite-Vec Backend

## 1. Data at Rest
- **Postgres**: Relies on file system permissions of the data dir.
- **SQLite**: Relies on file system permissions of the `.db` file.
- **Risk**: `maproom.db` might contain sensitive code snippets.
- **Mitigation**: Ensure the file is created with `0600` permissions (user read/write only).

## 2. Injection Attacks
- **SQL Injection**: `rusqlite` supports parameterized queries just like `tokio-postgres`.
- **Mitigation**: Strictly enforce parameterized queries. Never use `format!("SELECT ... {}", user_input)`.

## 3. Memory Safety
- **C Extension**: `sqlite-vec` is written in C.
- **Risk**: Buffer overflows in the C extension.
- **Mitigation**:
  - Use a pinned, reviewed version of `sqlite-vec`.
  - Run tests with AddressSanitizer (ASAN) if possible (advanced).
  - Rely on the fact that `sqlite-vec` is becoming standard.

## 4. Dependencies
- **Supply Chain**: Vendoring `sqlite-vec.c` prevents upstream changes breaking the build, but requires manual updates. This is a security trade-off (stability vs patching). We choose **Vendoring** for build reproducibility.
