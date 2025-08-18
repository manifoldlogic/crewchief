# CrewChief Web UI Authentication System

This document describes the complete authentication and authorization system implemented for the CrewChief Web UI.

## Features Implemented

✅ **Complete Authentication System**
- JWT tokens with 24-hour expiration (as required)
- Refresh token system with automatic rotation
- OAuth2 integration with GitHub and Google
- Role-based access control (RBAC) with admin, user, and viewer roles
- Secure session management with Redis
- Password complexity enforcement
- Account lockout after failed attempts
- Rate limiting on authentication endpoints
- CSRF protection enabled
- Secure cookie configuration

## Architecture

### Core Components

1. **Authentication Controllers** (`src/server/auth/controllers/`)
   - User registration, login, logout
   - Token refresh and session management
   - OAuth callback handling

2. **User Model** (`src/server/auth/models/user.ts`)
   - User CRUD operations with bcrypt password hashing
   - Role and permission management
   - Account lockout mechanisms

3. **Services** (`src/server/auth/services/`)
   - **JWT Service**: Token generation, validation, and rotation
   - **OAuth Service**: GitHub and Google OAuth integration
   - **Session Service**: Redis-based session management

4. **Security Middleware** (`src/server/auth/middleware/`)
   - Rate limiting and account lockout
   - CSRF protection and secure headers
   - Input validation

## Database Schema

### Users Table (`auth_users`)
```sql
- id, uuid, email, username, password_hash
- first_name, last_name, avatar_url
- is_active, is_verified, is_locked
- failed_login_attempts, locked_until
- two_factor_enabled, backup_codes
- preferences (JSONB)
```

### Roles & Permissions (`auth_roles`, `auth_user_roles`)
```sql
- Built-in roles: admin, user, viewer
- Flexible permission system
- Role assignment with expiration
```

### OAuth Integration (`auth_oauth_providers`, `auth_oauth_accounts`)
```sql
- GitHub and Google provider support
- Account linking capabilities
- Token storage and refresh
```

### Refresh Tokens (`auth_refresh_tokens`)
```sql
- Token rotation and family tracking
- Device information and IP tracking
- Automatic expiration and cleanup
```

## API Endpoints

### Authentication Routes (`/auth`)

#### Registration & Login
- `POST /auth/register` - User registration
- `POST /auth/login` - User login
- `POST /auth/logout` - User logout
- `POST /auth/logout-all` - Logout from all devices

#### Token Management
- `POST /auth/refresh` - Refresh access token
- `GET /auth/me` - Get current user info
- `PUT /auth/change-password` - Change password

#### OAuth
- `GET /auth/oauth/github` - GitHub OAuth initiation
- `GET /auth/oauth/github/callback` - GitHub OAuth callback
- `GET /auth/oauth/google` - Google OAuth initiation
- `GET /auth/oauth/google/callback` - Google OAuth callback
- `GET /auth/oauth/accounts` - List linked OAuth accounts
- `DELETE /auth/oauth/accounts/:providerId` - Unlink OAuth account

#### Session Management
- `GET /auth/sessions` - List user sessions
- `DELETE /auth/sessions/:sessionId` - Terminate specific session

#### Admin
- `GET /auth/admin/stats` - Authentication statistics (admin only)

## Security Features

### Password Security
- **Bcrypt hashing** with 12 salt rounds
- **Password complexity requirements**:
  - Minimum 8 characters
  - At least one uppercase letter
  - At least one lowercase letter
  - At least one number
  - At least one special character

### Account Protection
- **Account lockout** after 5 failed attempts for 30 minutes
- **Rate limiting**:
  - Login: 10 attempts per 15 minutes per IP
  - Registration: 5 attempts per hour per IP
  - Password reset: 3 attempts per hour per IP
- **Progressive delays** for repeated failed attempts

### Token Security
- **JWT tokens** expire after 24 hours (as required)
- **Refresh tokens** with 7-day expiration and rotation
- **Token family tracking** to detect reuse attacks
- **Automatic token cleanup** of expired tokens

### Session Security
- **Redis-based sessions** with secure configuration
- **HTTP-only cookies** to prevent XSS
- **Secure flag** for HTTPS-only transmission
- **SameSite protection** against CSRF

### CSRF Protection
- **Double-submit cookie pattern**
- **Origin validation**
- **Custom CSRF tokens** for state-changing operations

### Headers & CORS
- **Security headers**: CSP, HSTS, X-Frame-Options, etc.
- **CORS configuration** with specific allowed origins
- **Content type validation**

## Configuration

### Environment Variables
Copy `.env.example` to `.env` and configure:

```bash
# Required for production
JWT_SECRET=your-super-secret-jwt-key-at-least-32-characters-long
JWT_REFRESH_SECRET=your-super-secret-refresh-key-at-least-32-characters-long
SESSION_SECRET=your-super-secret-session-key-at-least-32-characters-long

# Database
DATABASE_URL=postgresql://username:password@localhost:5432/crewchief_web

# Redis (for sessions)
REDIS_URL=redis://localhost:6379

# OAuth (optional)
GITHUB_CLIENT_ID=your-github-client-id
GITHUB_CLIENT_SECRET=your-github-client-secret
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
```

### OAuth Setup

#### GitHub OAuth
1. Go to GitHub Settings > Developer settings > OAuth Apps
2. Create a new OAuth App with:
   - Authorization callback URL: `http://localhost:3456/auth/oauth/github/callback`
3. Copy Client ID and Client Secret to environment variables

#### Google OAuth
1. Go to Google Cloud Console > APIs & Services > Credentials
2. Create OAuth 2.0 Client IDs with:
   - Authorized redirect URI: `http://localhost:3456/auth/oauth/google/callback`
3. Copy Client ID and Client Secret to environment variables

## Database Setup

Run the migrations to set up the authentication tables:

```bash
pnpm db:migrate
```

This will create:
- `auth_users` - User accounts
- `auth_roles` - System roles (admin, user, viewer)
- `auth_user_roles` - User-role assignments
- `auth_refresh_tokens` - JWT refresh tokens
- `auth_oauth_providers` - OAuth provider configurations
- `auth_oauth_accounts` - User OAuth account links
- `auth_login_attempts` - Login attempt audit log
- `auth_rate_limits` - Rate limiting enforcement

## Role-Based Access Control (RBAC)

### Built-in Roles

1. **Admin** (`admin`)
   - Full system access with all permissions (`["*"]`)
   - Cannot be deleted (system role)

2. **User** (`user`)
   - Standard user access with basic permissions
   - Default role for new users
   - Permissions: agents:*, worktrees:*, runs:*, maproom:search, profile:*

3. **Viewer** (`viewer`)
   - Read-only access to view system status
   - Permissions: agents:read, worktrees:read, runs:read, maproom:search, profile:read

### Permission System
- Permissions follow the pattern: `resource:action`
- Examples: `agents:read`, `worktrees:create`, `runs:update`
- Admin role grants all permissions via wildcard (`*`)
- Permissions can be checked in middleware: `requirePermissions(['agents:create'])`

## Usage Examples

### Registration
```javascript
const response = await fetch('/auth/register', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'SecurePass123!',
    confirmPassword: 'SecurePass123!',
    firstName: 'John',
    lastName: 'Doe'
  })
});
```

### Login
```javascript
const response = await fetch('/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  credentials: 'include', // Important for cookies
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'SecurePass123!'
  })
});
```

### Making Authenticated Requests
```javascript
// Cookies are automatically included
const response = await fetch('/api/agents', {
  credentials: 'include'
});

// Or use Authorization header
const response = await fetch('/api/agents', {
  headers: {
    'Authorization': `Bearer ${accessToken}`
  }
});
```

## Monitoring & Administration

### Authentication Statistics
Admin users can view authentication statistics:
- Active sessions and tokens
- Login attempts and failures
- OAuth usage statistics
- Rate limiting status

### Session Management
Users can:
- View all active sessions
- Terminate specific sessions
- Logout from all devices

### Security Monitoring
The system logs:
- All login attempts (successful and failed)
- Account lockouts and unlocks
- Token refresh events
- OAuth authentication flows

## Development Notes

### Testing Authentication
Use the development bypass header for testing:
```bash
curl -H "x-bypass-auth: true" http://localhost:3456/api/protected
```

### Password Requirements
Passwords must contain:
- At least 8 characters
- One uppercase letter (A-Z)
- One lowercase letter (a-z)
- One number (0-9)
- One special character (@$!%*?&)

### Token Expiration
- **Access tokens**: 24 hours (as required)
- **Refresh tokens**: 7 days with automatic rotation
- **Sessions**: 24 hours with rolling expiration

## Troubleshooting

### Common Issues

1. **"Account is locked"**
   - Wait 30 minutes or contact admin to unlock
   - Check failed login attempts in logs

2. **OAuth callback errors**
   - Verify OAuth app configuration
   - Check redirect URLs match exactly
   - Ensure environment variables are set

3. **Token refresh failures**
   - May indicate token reuse attack (family revoked)
   - User needs to login again

4. **Session issues**
   - Ensure Redis is running and accessible
   - Check Redis connection URL
   - Verify session cookie settings

### Redis Connection
Ensure Redis is running:
```bash
redis-server
# or with Docker
docker run -p 6379:6379 redis:alpine
```

## Security Considerations

1. **Production Deployment**
   - Use strong, unique secrets (at least 32 characters)
   - Enable HTTPS (secure cookies require it)
   - Configure proper CORS origins
   - Set up Redis with authentication
   - Use environment variables for all secrets

2. **Database Security**
   - Passwords are hashed with bcrypt (12 rounds)
   - Sensitive OAuth tokens should be encrypted at rest
   - Regular cleanup of expired tokens and sessions

3. **Rate Limiting**
   - Adjust limits based on your use case
   - Monitor for unusual patterns
   - Consider IP whitelisting for admin access

This authentication system provides enterprise-grade security while maintaining usability and developer experience.