-- MCP Test Fixtures
-- Fixture Version: 1.0.0
-- Compatible Schema: migrations 0000-0020
-- Generated: 2025-11-25T06:30:00Z
-- Generator: packages/maproom-mcp/scripts/create-test-fixtures.sh (hand-crafted)
--
-- Test corpus: packages/maproom-mcp/tests/corpus/
-- Files: 7
-- Chunks: 48
--
-- This fixture provides pre-indexed test data for deterministic integration tests.
-- It includes TypeScript, Python, Rust, and Markdown samples with known query results.
--
-- Usage:
--   psql $MAPROOM_DATABASE_URL < tests/setup/test-fixtures.sql
--
-- Query->Result Expectations:
--   See tests/corpus/README.md for the full 12 query->result matrix

BEGIN;

-- Temporarily disable triggers for faster loading
SET session_replication_role = replica;

-- Clean up any existing test data
DELETE FROM maproom.chunks WHERE file_id IN (
  SELECT f.id FROM maproom.files f
  JOIN maproom.repos r ON f.repo_id = r.id
  WHERE r.name = 'test-corpus'
);
DELETE FROM maproom.files WHERE repo_id IN (
  SELECT id FROM maproom.repos WHERE name = 'test-corpus'
);
DELETE FROM maproom.commits WHERE repo_id IN (
  SELECT id FROM maproom.repos WHERE name = 'test-corpus'
);
DELETE FROM maproom.worktrees WHERE repo_id IN (
  SELECT id FROM maproom.repos WHERE name = 'test-corpus'
);
DELETE FROM maproom.repos WHERE name = 'test-corpus';

-- ============================================================================
-- Repository
-- ============================================================================

INSERT INTO maproom.repos (id, name, root_path)
VALUES (1000, 'test-corpus', '/workspace/packages/maproom-mcp/tests/corpus');

-- ============================================================================
-- Worktree
-- ============================================================================

INSERT INTO maproom.worktrees (id, repo_id, name, abs_path)
VALUES (1000, 1000, 'main', '/workspace/packages/maproom-mcp/tests/corpus');

-- ============================================================================
-- Commit
-- ============================================================================

INSERT INTO maproom.commits (id, repo_id, sha, committed_at)
VALUES (1000, 1000, 'fixture-commit-sha', '2025-11-25 00:00:00+00');

-- ============================================================================
-- Files
-- ============================================================================

INSERT INTO maproom.files (id, repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes, last_modified) VALUES
(1001, 1000, 1000, 1000, 'typescript/auth-service.ts', 'ts', 'hash_auth_service', 2048, '2025-11-25 00:00:00+00'),
(1002, 1000, 1000, 1000, 'typescript/database-client.ts', 'ts', 'hash_database_client', 2560, '2025-11-25 00:00:00+00'),
(1003, 1000, 1000, 1000, 'python/validate_token.py', 'py', 'hash_validate_token', 1920, '2025-11-25 00:00:00+00'),
(1004, 1000, 1000, 1000, 'python/user_service.py', 'py', 'hash_user_service', 3072, '2025-11-25 00:00:00+00'),
(1005, 1000, 1000, 1000, 'rust/database.rs', 'rs', 'hash_database_rs', 2816, '2025-11-25 00:00:00+00'),
(1006, 1000, 1000, 1000, 'rust/config.rs', 'rs', 'hash_config_rs', 2944, '2025-11-25 00:00:00+00'),
(1007, 1000, 1000, 1000, 'markdown/api-docs.md', 'md', 'hash_api_docs', 2688, '2025-11-25 00:00:00+00');

-- ============================================================================
-- Chunks
-- ============================================================================
-- Each chunk maps to a specific query in the query->result matrix

-- TypeScript: auth-service.ts chunks
INSERT INTO maproom.chunks (id, file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, blob_sha, worktree_ids) VALUES
-- Query 2: "user authentication" -> AuthService class
(1001, 1001, 'AuthService', 'class', 'export class AuthService', 'AuthService provides user authentication and token management. Use this service for all login, logout, and session operations.', 22, 68, 'export class AuthService { private config: AuthConfig; constructor(config: AuthConfig) { this.config = config; } async authenticate(username: string, password: string): Promise<AuthResult> { ... } }',
  to_tsvector('english', 'AuthService user authentication token management login logout session authenticate validateToken'),
  1.0, 0.0, 'blob_authservice_class', '[]'),

-- Query 1: "authenticate" -> AuthService.authenticate()
(1002, 1001, 'authenticate', 'func', 'async authenticate(username: string, password: string): Promise<AuthResult>', 'Authenticate a user with username and password. Returns a JWT token on successful authentication.', 33, 41, 'async authenticate(username: string, password: string): Promise<AuthResult> { if (!username || !password) { return { success: false, error: ''Invalid credentials'' }; } const token = this.generateToken(username); return { success: true, token, userId: username }; }',
  to_tsvector('english', 'authenticate user username password JWT token authentication credentials verify login'),
  1.0, 0.0, 'blob_authenticate_func', '[]'),

-- Query 4: "validateToken" -> AuthService.validateToken()
(1003, 1001, 'validateToken', 'func', 'validateToken(token: string): boolean', 'Validate an authentication token. Checks expiry and signature validity.', 47, 52, 'validateToken(token: string): boolean { if (!token) return false; return token.length > 0; }',
  to_tsvector('english', 'validateToken validate token authentication expiry signature validity check'),
  1.0, 0.0, 'blob_validatetoken_func', '[]'),

(1004, 1001, 'AuthConfig', 'type', 'export interface AuthConfig', 'Configuration for authentication service', 6, 9, 'export interface AuthConfig { tokenExpiry: number; secretKey: string; }',
  to_tsvector('english', 'AuthConfig configuration token expiry secret key'),
  1.0, 0.0, 'blob_authconfig_type', '[]'),

(1005, 1001, 'AuthResult', 'type', 'export interface AuthResult', 'Result of authentication operation', 11, 16, 'export interface AuthResult { success: boolean; token?: string; userId?: string; error?: string; }',
  to_tsvector('english', 'AuthResult result authentication success token userId error'),
  1.0, 0.0, 'blob_authresult_type', '[]'),

(1006, 1001, 'refreshToken', 'func', 'refreshToken(oldToken: string): string | null', 'Refresh an existing authentication token.', 57, 62, 'refreshToken(oldToken: string): string | null { if (!this.validateToken(oldToken)) { return null; } return this.generateToken(''refreshed''); }',
  to_tsvector('english', 'refreshToken refresh token authentication'),
  1.0, 0.0, 'blob_refreshtoken_func', '[]');

-- TypeScript: database-client.ts chunks
INSERT INTO maproom.chunks (id, file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, blob_sha, worktree_ids) VALUES
-- Query 6: "connect to database" -> DatabaseClient.connect()
(1010, 1002, 'DatabaseClient', 'class', 'export class DatabaseClient', 'DatabaseClient manages database connections and query execution. Supports connection pooling and automatic reconnection.', 23, 90, 'export class DatabaseClient { private config: DatabaseConfig; private connected: boolean = false; constructor(config: DatabaseConfig) { this.config = config; } async connect(): Promise<void> { ... } async query<T>(sql: string, params?: unknown[]): Promise<QueryResult<T>> { ... } }',
  to_tsvector('english', 'DatabaseClient database client connection query execution pooling reconnection PostgreSQL'),
  1.0, 0.0, 'blob_databaseclient_class', '[]'),

(1011, 1002, 'connect', 'func', 'async connect(): Promise<void>', 'Connect to the database server. Establishes a connection pool for efficient query execution.', 35, 40, 'async connect(): Promise<void> { console.log(''Connecting to database at '' + this.config.host + '':'' + this.config.port); this.connected = true; }',
  to_tsvector('english', 'connect database server connection pool establish PostgreSQL'),
  1.0, 0.0, 'blob_connect_func', '[]'),

-- Query 7: "query data" -> DatabaseClient.query()
(1012, 1002, 'query', 'func', 'async query<T>(sql: string, params?: unknown[]): Promise<QueryResult<T>>', 'Execute a SQL query and return results. Supports parameterized queries to prevent SQL injection.', 46, 53, 'async query<T = unknown>(sql: string, params?: unknown[]): Promise<QueryResult<T>> { if (!this.connected) { throw new Error(''Not connected to database''); } return { rows: [], rowCount: 0 }; }',
  to_tsvector('english', 'query SQL execute data results parameterized injection prevention'),
  1.0, 0.0, 'blob_query_func', '[]'),

(1013, 1002, 'DatabaseConfig', 'type', 'export interface DatabaseConfig', 'Configuration for database connections', 6, 12, 'export interface DatabaseConfig { host: string; port: number; database: string; user: string; password: string; }',
  to_tsvector('english', 'DatabaseConfig configuration database host port user password'),
  1.0, 0.0, 'blob_databaseconfig_type', '[]'),

(1014, 1002, 'QueryResult', 'type', 'export interface QueryResult<T>', 'Result type for database queries', 14, 17, 'export interface QueryResult<T = unknown> { rows: T[]; rowCount: number; }',
  to_tsvector('english', 'QueryResult result query rows rowCount'),
  1.0, 0.0, 'blob_queryresult_type', '[]'),

(1015, 1002, 'execute', 'func', 'async execute(sql: string, params?: unknown[]): Promise<number>', 'Execute a query that modifies data (INSERT, UPDATE, DELETE).', 58, 61, 'async execute(sql: string, params?: unknown[]): Promise<number> { const result = await this.query(sql, params); return result.rowCount; }',
  to_tsvector('english', 'execute SQL INSERT UPDATE DELETE modify data'),
  1.0, 0.0, 'blob_execute_func', '[]'),

(1016, 1002, 'beginTransaction', 'func', 'async beginTransaction(): Promise<void>', 'Begin a database transaction.', 66, 68, 'async beginTransaction(): Promise<void> { await this.query(''BEGIN''); }',
  to_tsvector('english', 'beginTransaction transaction BEGIN database'),
  1.0, 0.0, 'blob_begintransaction_func', '[]'),

(1017, 1002, 'commit', 'func', 'async commit(): Promise<void>', 'Commit the current transaction.', 73, 75, 'async commit(): Promise<void> { await this.query(''COMMIT''); }',
  to_tsvector('english', 'commit transaction COMMIT database'),
  1.0, 0.0, 'blob_commit_func', '[]'),

(1018, 1002, 'disconnect', 'func', 'async disconnect(): Promise<void>', 'Close the database connection.', 87, 89, 'async disconnect(): Promise<void> { this.connected = false; }',
  to_tsvector('english', 'disconnect close database connection'),
  1.0, 0.0, 'blob_disconnect_func', '[]');

-- Python: validate_token.py chunks
INSERT INTO maproom.chunks (id, file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, blob_sha, worktree_ids) VALUES
-- Query 3: "validate_token" -> validate_token() function
(1020, 1003, 'validate_token', 'func', 'def validate_token(token: str, secret_key: str = "default_secret") -> bool', 'Validate an authentication token. Args: token - The JWT token to validate, secret_key - The secret key for signature verification. Returns: True if the token is valid, False otherwise', 47, 59, 'def validate_token(token: str, secret_key: str = "default_secret") -> bool: validator = TokenValidator(secret_key) return validator.validate(token)',
  to_tsvector('english', 'validate_token validate token authentication JWT secret key signature verification'),
  1.0, 0.0, 'blob_validate_token_func', '[]'),

(1021, 1003, 'TokenValidator', 'class', 'class TokenValidator', 'TokenValidator handles JWT token validation and verification. Supports signature verification and expiration checking.', 19, 44, 'class TokenValidator: def __init__(self, secret_key: str): self.secret_key = secret_key def validate(self, token: str) -> bool: ... def decode(self, token: str) -> Optional[TokenPayload]: ...',
  to_tsvector('english', 'TokenValidator token validation verification signature expiration JWT'),
  1.0, 0.0, 'blob_tokenvalidator_class', '[]'),

(1022, 1003, 'TokenPayload', 'class', '@dataclass class TokenPayload', 'Decoded token payload containing user information.', 11, 16, '@dataclass class TokenPayload: user_id: str issued_at: int expires_at: int',
  to_tsvector('english', 'TokenPayload payload token user_id issued_at expires_at'),
  1.0, 0.0, 'blob_tokenpayload_class', '[]'),

(1023, 1003, 'decode_token', 'func', 'def decode_token(token: str) -> Optional[TokenPayload]', 'Decode a JWT token without validation. Args: token - The JWT token to decode. Returns: TokenPayload if decoding succeeds, None otherwise', 62, 73, 'def decode_token(token: str) -> Optional[TokenPayload]: validator = TokenValidator("") return validator.decode(token)',
  to_tsvector('english', 'decode_token decode JWT token payload'),
  1.0, 0.0, 'blob_decode_token_func', '[]'),

(1024, 1003, 'is_token_expired', 'func', 'def is_token_expired(token: str) -> bool', 'Check if a token has expired.', 76, 81, 'def is_token_expired(token: str) -> bool: payload = decode_token(token) if payload is None: return True return payload.expires_at < time.time()',
  to_tsvector('english', 'is_token_expired token expired expiration check'),
  1.0, 0.0, 'blob_is_token_expired_func', '[]');

-- Python: user_service.py chunks
INSERT INTO maproom.chunks (id, file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, blob_sha, worktree_ids) VALUES
-- Query 8: "user CRUD" -> UserService class
(1030, 1004, 'UserService', 'class', 'class UserService', 'UserService handles all user CRUD operations. Provides methods to create, read, update, and delete user accounts.', 19, 108, 'class UserService: def __init__(self, database): self.database = database def create_user(self, username: str, email: str) -> User: ... def get_user(self, user_id: str) -> Optional[User]: ... def update_user(...) -> Optional[User]: ... def delete_user(self, user_id: str) -> bool: ...',
  to_tsvector('english', 'UserService user CRUD create read update delete operations accounts'),
  1.0, 0.0, 'blob_userservice_class', '[]'),

-- Query 11: "get user by id" -> UserService.get_user()
(1031, 1004, 'get_user', 'func', 'def get_user(self, user_id: str) -> Optional[User]', 'Get a user by their ID. Args: user_id - The unique identifier of the user. Returns: User object if found, None otherwise', 51, 61, 'def get_user(self, user_id: str) -> Optional[User]: return self._users.get(user_id)',
  to_tsvector('english', 'get_user user id identifier find retrieve'),
  1.0, 0.0, 'blob_get_user_func', '[]'),

(1032, 1004, 'User', 'class', '@dataclass class User', 'User data model representing an account in the system.', 10, 16, '@dataclass class User: id: str username: str email: str created_at: int',
  to_tsvector('english', 'User data model account username email'),
  1.0, 0.0, 'blob_user_class', '[]'),

(1033, 1004, 'create_user', 'func', 'def create_user(self, username: str, email: str) -> User', 'Create a new user account. Args: username - The username for the new account, email - The email address. Returns: The created User object', 29, 49, 'def create_user(self, username: str, email: str) -> User: user_id = f"user_{len(self._users) + 1}" user = User(id=user_id, username=username, email=email, created_at=int(time.time())) self._users[user_id] = user return user',
  to_tsvector('english', 'create_user create user account username email'),
  1.0, 0.0, 'blob_create_user_func', '[]'),

(1034, 1004, 'update_user', 'func', 'def update_user(self, user_id: str, username: str = None, email: str = None) -> Optional[User]', 'Update a user information. Args: user_id - The ID of the user to update, username - New username (optional), email - New email (optional). Returns: Updated User object if found, None otherwise', 70, 89, 'def update_user(self, user_id: str, username: str = None, email: str = None) -> Optional[User]: user = self.get_user(user_id) if user is None: return None if username: user.username = username if email: user.email = email return user',
  to_tsvector('english', 'update_user update user modify username email'),
  1.0, 0.0, 'blob_update_user_func', '[]'),

(1035, 1004, 'delete_user', 'func', 'def delete_user(self, user_id: str) -> bool', 'Delete a user account. Args: user_id - The ID of the user to delete. Returns: True if deleted, False if user not found', 91, 104, 'def delete_user(self, user_id: str) -> bool: if user_id in self._users: del self._users[user_id] return True return False',
  to_tsvector('english', 'delete_user delete user account remove'),
  1.0, 0.0, 'blob_delete_user_func', '[]'),

(1036, 1004, 'list_users', 'func', 'def list_users(self) -> List[User]', 'Get all users in the system.', 106, 108, 'def list_users(self) -> List[User]: return list(self._users.values())',
  to_tsvector('english', 'list_users list all users'),
  1.0, 0.0, 'blob_list_users_func', '[]'),

(1037, 1004, 'get_user_by_email', 'func', 'def get_user_by_email(self, email: str) -> Optional[User]', 'Find a user by their email address.', 63, 68, 'def get_user_by_email(self, email: str) -> Optional[User]: for user in self._users.values(): if user.email == email: return user return None',
  to_tsvector('english', 'get_user_by_email user email find'),
  1.0, 0.0, 'blob_get_user_by_email_func', '[]');

-- Rust: database.rs chunks
INSERT INTO maproom.chunks (id, file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, blob_sha, worktree_ids) VALUES
-- Query 5: "DatabaseConnection" -> DatabaseConnection struct
(1040, 1005, 'DatabaseConnection', 'class', 'pub struct DatabaseConnection', 'DatabaseConnection manages a PostgreSQL database connection. Supports connection pooling and automatic reconnection.', 33, 36, 'pub struct DatabaseConnection { config: DatabaseConfig, connected: bool, }',
  to_tsvector('english', 'DatabaseConnection database connection PostgreSQL pooling reconnection struct'),
  1.0, 0.0, 'blob_databaseconnection_struct', '[]'),

-- Query 12: "impl Connection" -> impl Connection for DatabaseConnection
(1041, 1005, 'impl Connection for DatabaseConnection', 'module', 'impl Connection for DatabaseConnection', 'Implementation of Connection trait for DatabaseConnection', 65, 78, 'impl Connection for DatabaseConnection { fn execute(&self, query: &str) -> DbResult<Vec<String>> { self.query(query, &[]) } fn is_connected(&self) -> bool { self.connected } fn close(&mut self) -> DbResult<()> { self.connected = false; Ok(()) } }',
  to_tsvector('english', 'impl Connection DatabaseConnection trait implementation execute is_connected close'),
  1.0, 0.0, 'blob_impl_connection', '[]'),

(1042, 1005, 'Connection', 'type', 'pub trait Connection', 'Trait defining database connection behavior.', 20, 29, 'pub trait Connection { fn execute(&self, query: &str) -> DbResult<Vec<String>>; fn is_connected(&self) -> bool; fn close(&mut self) -> DbResult<()>; }',
  to_tsvector('english', 'Connection trait database behavior execute is_connected close'),
  1.0, 0.0, 'blob_connection_trait', '[]'),

(1043, 1005, 'DatabaseConfig', 'type', 'pub struct DatabaseConfig', 'Configuration for database connections.', 8, 14, 'pub struct DatabaseConfig { pub host: String, pub port: u16, pub database: String, pub user: String, pub password: String, }',
  to_tsvector('english', 'DatabaseConfig configuration database host port user password struct'),
  1.0, 0.0, 'blob_databaseconfig_struct', '[]'),

(1044, 1005, 'connect', 'func', 'pub fn connect(&mut self) -> DbResult<()>', 'Connect to the database server.', 48, 53, 'pub fn connect(&mut self) -> DbResult<()> { println!("Connecting to {}:{}", self.config.host, self.config.port); self.connected = true; Ok(()) }',
  to_tsvector('english', 'connect database server'),
  1.0, 0.0, 'blob_rust_connect_func', '[]'),

(1045, 1005, 'query', 'func', 'pub fn query(&self, sql: &str, _params: &[&str]) -> DbResult<Vec<String>>', 'Execute a parameterized query.', 56, 62, 'pub fn query(&self, sql: &str, _params: &[&str]) -> DbResult<Vec<String>> { if !self.connected { return Err("Not connected".into()); } Ok(vec![]) }',
  to_tsvector('english', 'query SQL parameterized execute'),
  1.0, 0.0, 'blob_rust_query_func', '[]');

-- Rust: config.rs chunks
INSERT INTO maproom.chunks (id, file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, blob_sha, worktree_ids) VALUES
-- Query 9: "configuration loading" -> load_config()
(1050, 1006, 'load_config', 'func', 'pub fn load_config(path: &str) -> Result<Config, Box<dyn Error>>', 'Load configuration from a file path. Supports JSON and TOML configuration files.', 51, 66, 'pub fn load_config(path: &str) -> Result<Config, Box<dyn Error>> { let path = Path::new(path); if !path.exists() { return Err(format!("Configuration file not found: {}", path.display()).into()); } let mut config = Config::default(); println!("Loading configuration from {}", path.display()); Ok(config) }',
  to_tsvector('english', 'load_config load configuration file JSON TOML parse'),
  1.0, 0.0, 'blob_load_config_func', '[]'),

(1051, 1006, 'Config', 'class', 'pub struct Config', 'Application configuration settings.', 10, 19, 'pub struct Config { pub database_url: String, pub port: u16, pub log_level: String, pub settings: HashMap<String, String>, }',
  to_tsvector('english', 'Config configuration settings database_url port log_level struct'),
  1.0, 0.0, 'blob_config_struct', '[]'),

(1052, 1006, 'load_from_env', 'func', 'pub fn load_from_env() -> Config', 'Load configuration from environment variables.', 69, 90, 'pub fn load_from_env() -> Config { let mut config = Config::default(); if let Ok(url) = std::env::var("DATABASE_URL") { config.database_url = url; } if let Ok(port) = std::env::var("PORT") { if let Ok(p) = port.parse() { config.port = p; } } if let Ok(level) = std::env::var("LOG_LEVEL") { config.log_level = level; } config }',
  to_tsvector('english', 'load_from_env load configuration environment variables DATABASE_URL PORT LOG_LEVEL'),
  1.0, 0.0, 'blob_load_from_env_func', '[]'),

(1053, 1006, 'validate_config', 'func', 'pub fn validate_config(config: &Config) -> Result<(), String>', 'Validate configuration values.', 93, 101, 'pub fn validate_config(config: &Config) -> Result<(), String> { if config.database_url.is_empty() { return Err("database_url is required".to_string()); } if config.port == 0 { return Err("port must be greater than 0".to_string()); } Ok(()) }',
  to_tsvector('english', 'validate_config validate configuration values'),
  1.0, 0.0, 'blob_validate_config_func', '[]');

-- Markdown: api-docs.md chunks
INSERT INTO maproom.chunks (id, file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, blob_sha, worktree_ids) VALUES
-- Query 10: "API documentation" -> api-docs.md heading
(1060, 1007, 'API Documentation', 'other', '# API Documentation', 'This document provides comprehensive API documentation for the application services.', 1, 4, '# API Documentation\n\nThis document provides comprehensive API documentation for the application services.',
  to_tsvector('english', 'API documentation services comprehensive'),
  1.0, 0.0, 'blob_api_docs_h1', '[]'),

(1061, 1007, 'Authentication API', 'other', '## Authentication API', 'The authentication API handles user login, logout, and session management.', 6, 28, '## Authentication API\n\nThe authentication API handles user login, logout, and session management.\n\n### POST /auth/login\n\nAuthenticate a user with username and password.',
  to_tsvector('english', 'Authentication API login logout session management'),
  1.0, 0.0, 'blob_auth_api_section', '[]'),

(1062, 1007, 'User API', 'other', '## User API', 'The user API provides CRUD operations for user accounts.', 41, 71, '## User API\n\nThe user API provides CRUD operations for user accounts.\n\n### GET /users/:id\n\nRetrieve a user by their ID.',
  to_tsvector('english', 'User API CRUD operations accounts'),
  1.0, 0.0, 'blob_user_api_section', '[]'),

(1063, 1007, 'Database API', 'other', '## Database API', 'Internal API for database operations.', 73, 83, '## Database API\n\nInternal API for database operations.\n\n### Connection Management\n\nDatabase connections are pooled for optimal performance.',
  to_tsvector('english', 'Database API internal operations connection management'),
  1.0, 0.0, 'blob_database_api_section', '[]'),

(1064, 1007, 'Configuration', 'other', '## Configuration', 'Environment Variables and Configuration Files', 85, 97, '## Configuration\n\n### Environment Variables\n\n| Variable | Description | Default |\n|----------|-------------|---------|\n| DATABASE_URL | PostgreSQL connection string | localhost:5432 |',
  to_tsvector('english', 'Configuration environment variables DATABASE_URL PORT LOG_LEVEL'),
  1.0, 0.0, 'blob_config_section', '[]');

-- Re-enable triggers
SET session_replication_role = DEFAULT;

-- Update sequences to avoid conflicts with future inserts
SELECT setval('maproom.repos_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM maproom.repos), 1100));
SELECT setval('maproom.worktrees_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM maproom.worktrees), 1100));
SELECT setval('maproom.commits_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM maproom.commits), 1100));
SELECT setval('maproom.files_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM maproom.files), 1100));
SELECT setval('maproom.chunks_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM maproom.chunks), 1100));

COMMIT;

-- ============================================================================
-- Verification queries
-- ============================================================================
\echo ''
\echo '=== Test Fixture Statistics ==='
\echo ''

SELECT
  f.language,
  COUNT(*) as chunk_count
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
JOIN maproom.repos r ON f.repo_id = r.id
WHERE r.name = 'test-corpus'
GROUP BY f.language
ORDER BY chunk_count DESC;

\echo ''

SELECT
  c.kind::text,
  COUNT(*) as count
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
JOIN maproom.repos r ON f.repo_id = r.id
WHERE r.name = 'test-corpus'
GROUP BY c.kind
ORDER BY count DESC
LIMIT 10;

\echo ''

SELECT
  COUNT(*) as total_chunks,
  COUNT(CASE WHEN code_embedding IS NOT NULL THEN 1 END) as with_code_emb,
  COUNT(CASE WHEN text_embedding IS NOT NULL THEN 1 END) as with_text_emb
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
JOIN maproom.repos r ON f.repo_id = r.id
WHERE r.name = 'test-corpus';

\echo ''
\echo 'Test fixtures loaded successfully!'
