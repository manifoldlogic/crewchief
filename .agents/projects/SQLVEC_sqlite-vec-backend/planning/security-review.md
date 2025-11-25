# Security Review: SQLite-Vec Backend

## 1. File Permissions
- **SQLite**: Database is a file (`~/.config/crewchief/maproom.db`).
  - *Risk*: Other users on the system reading the DB.
  - *Mitigation*: Set file permissions to `600` (owner read/write only).

## 2. SQL Injection
- **Risk**: Dynamic SQL generation for different dialects.
- **Mitigation**: Always use parameterized queries (`?` for SQLite, `$1` for Postgres). Never string concatenation.

## 3. Extension Security
- **SQLite Extensions**: Loading C extensions can be risky.
- **Mitigation**: Statically link `sqlite-vec` during build time. Do not allow loading arbitrary shared libraries at runtime.

## 4. Data Isolation
- **Postgres**: Relies on DB user permissions.
- **SQLite**: Relies on filesystem permissions. This is generally sufficient for a personal CLI tool.

