# API Documentation

This document provides comprehensive API documentation for the application services.

## Authentication API

The authentication API handles user login, logout, and session management.

### POST /auth/login

Authenticate a user with username and password.

**Request Body:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response:**
```json
{
  "success": true,
  "token": "jwt_token_here",
  "userId": "user_123"
}
```

### POST /auth/logout

End the current user session.

**Headers:**
- `Authorization: Bearer <token>`

### GET /auth/validate

Validate the current authentication token.

## User API

The user API provides CRUD operations for user accounts.

### GET /users/:id

Retrieve a user by their ID.

**Parameters:**
- `id` - The unique user identifier

**Response:**
```json
{
  "id": "user_123",
  "username": "johndoe",
  "email": "john@example.com"
}
```

### POST /users

Create a new user account.

### PUT /users/:id

Update an existing user's information.

### DELETE /users/:id

Delete a user account.

## Database API

Internal API for database operations.

### Connection Management

Database connections are pooled for optimal performance. The connection pool automatically manages connection lifecycle.

### Query Execution

All queries support parameterized inputs to prevent SQL injection attacks.

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| DATABASE_URL | PostgreSQL connection string | localhost:5432 |
| PORT | Server port | 8080 |
| LOG_LEVEL | Logging verbosity | info |

### Configuration Files

Configuration can be loaded from JSON or TOML files using the `load_config()` function.
