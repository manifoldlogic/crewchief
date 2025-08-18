// Auth module exports
export { AuthController } from './controllers/auth.js';
export { UserModel, type User, type CreateUserData, type UpdateUserData, type Role } from './models/user.js';
export { JWTService, type TokenPair, type AccessTokenPayload, type RefreshTokenPayload } from './services/jwt.js';
export { OAuthService, type OAuthProvider, type OAuthAccount } from './services/oauth.js';
export { SessionService, type SessionData, type SessionInfo } from './services/session.js';
export { createAuthRouter } from './routes/auth.js';
export { 
  RateLimitService,
  createRateLimit,
  loginRateLimit,
  registerRateLimit,
  passwordResetRateLimit,
  refreshRateLimit,
  apiRateLimit,
  createAccountLockout,
  createProgressiveDelay,
  createIPSecurityCheck,
  createCSRFProtection,
} from './middleware/rate-limit.js';
export {
  csrfProtection,
  doubleSubmitCookie,
  validateOrigin,
  secureHeaders,
  secureCookies,
  csrfTokenRateLimit,
  cleanupCSRFSecrets,
  csrfTokenEndpoint,
  type CSRFProtectedRequest,
} from './middleware/csrf.js';
export {
  loginValidation,
  registerValidation,
  refreshValidation,
  changePasswordValidation,
  profileUpdateValidation,
} from './utils/validation.js';