import { body, ValidationChain } from 'express-validator';

// Email validation
export const validateEmail = (): ValidationChain =>
  body('email')
    .isEmail()
    .normalizeEmail()
    .withMessage('Please provide a valid email address')
    .isLength({ max: 255 })
    .withMessage('Email address too long');

// Password validation
export const validatePassword = (field = 'password'): ValidationChain =>
  body(field)
    .isLength({ min: 8, max: 128 })
    .withMessage('Password must be between 8 and 128 characters')
    .matches(/^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]/)
    .withMessage('Password must contain at least one uppercase letter, one lowercase letter, one number, and one special character (@$!%*?&)');

// Username validation
export const validateUsername = (): ValidationChain =>
  body('username')
    .optional()
    .isLength({ min: 3, max: 30 })
    .withMessage('Username must be between 3 and 30 characters')
    .matches(/^[a-zA-Z0-9_-]+$/)
    .withMessage('Username can only contain letters, numbers, underscores, and hyphens');

// Name validation
export const validateName = (field: string): ValidationChain =>
  body(field)
    .optional()
    .isLength({ min: 1, max: 100 })
    .withMessage(`${field} must be between 1 and 100 characters`)
    .matches(/^[a-zA-Z\s'-]+$/)
    .withMessage(`${field} can only contain letters, spaces, apostrophes, and hyphens`);

// Refresh token validation
export const validateRefreshToken = (): ValidationChain =>
  body('refreshToken')
    .notEmpty()
    .withMessage('Refresh token is required')
    .isJWT()
    .withMessage('Invalid refresh token format');

// Password confirmation validation
export const validatePasswordConfirmation = (): ValidationChain =>
  body('confirmPassword')
    .custom((value, { req }) => {
      if (value !== req.body.password) {
        throw new Error('Password confirmation does not match password');
      }
      return true;
    });

// Login validation schema
export const loginValidation = [
  validateEmail(),
  body('password')
    .notEmpty()
    .withMessage('Password is required'),
  body('rememberMe')
    .optional()
    .isBoolean()
    .withMessage('Remember me must be a boolean value'),
];

// Registration validation schema
export const registerValidation = [
  validateEmail(),
  validatePassword(),
  validatePasswordConfirmation(),
  validateUsername(),
  validateName('firstName'),
  validateName('lastName'),
];

// Refresh token validation schema
export const refreshValidation = [
  validateRefreshToken(),
];

// Change password validation schema
export const changePasswordValidation = [
  body('currentPassword')
    .notEmpty()
    .withMessage('Current password is required'),
  validatePassword('newPassword'),
  body('confirmPassword')
    .custom((value, { req }) => {
      if (value !== req.body.newPassword) {
        throw new Error('Password confirmation does not match new password');
      }
      return true;
    }),
];

// Password reset request validation
export const passwordResetRequestValidation = [
  validateEmail(),
];

// Password reset validation
export const passwordResetValidation = [
  body('token')
    .notEmpty()
    .withMessage('Reset token is required')
    .isLength({ min: 32, max: 255 })
    .withMessage('Invalid reset token'),
  validatePassword('newPassword'),
  validatePasswordConfirmation(),
];

// Email verification validation
export const emailVerificationValidation = [
  body('token')
    .notEmpty()
    .withMessage('Verification token is required')
    .isLength({ min: 32, max: 255 })
    .withMessage('Invalid verification token'),
];

// Profile update validation
export const profileUpdateValidation = [
  validateUsername(),
  validateName('firstName'),
  validateName('lastName'),
  body('avatarUrl')
    .optional()
    .isURL()
    .withMessage('Avatar URL must be a valid URL'),
  body('preferences')
    .optional()
    .isObject()
    .withMessage('Preferences must be an object'),
];

// Two-factor authentication setup validation
export const twoFactorSetupValidation = [
  body('secret')
    .notEmpty()
    .withMessage('2FA secret is required')
    .isBase32()
    .withMessage('Invalid 2FA secret format'),
  body('token')
    .notEmpty()
    .withMessage('2FA token is required')
    .matches(/^\d{6}$/)
    .withMessage('2FA token must be 6 digits'),
];

// Two-factor authentication verification validation
export const twoFactorVerifyValidation = [
  body('token')
    .notEmpty()
    .withMessage('2FA token is required')
    .matches(/^\d{6}$/)
    .withMessage('2FA token must be 6 digits'),
];

// Admin user creation validation
export const adminCreateUserValidation = [
  validateEmail(),
  body('tempPassword')
    .isLength({ min: 12 })
    .withMessage('Temporary password must be at least 12 characters'),
  validateUsername(),
  validateName('firstName'),
  validateName('lastName'),
  body('roles')
    .isArray()
    .withMessage('Roles must be an array')
    .custom((roles) => {
      if (roles.length === 0) {
        throw new Error('At least one role must be assigned');
      }
      return true;
    }),
];

// Role assignment validation
export const roleAssignmentValidation = [
  body('userId')
    .isInt({ min: 1 })
    .withMessage('Valid user ID is required'),
  body('roleId')
    .isInt({ min: 1 })
    .withMessage('Valid role ID is required'),
  body('expiresAt')
    .optional()
    .isISO8601()
    .withMessage('Expiration date must be in ISO 8601 format'),
];

// OAuth state validation
export const oauthStateValidation = [
  body('state')
    .optional()
    .isLength({ min: 10, max: 255 })
    .withMessage('OAuth state must be between 10 and 255 characters'),
];

// Device management validation
export const deviceManagementValidation = [
  body('deviceId')
    .notEmpty()
    .withMessage('Device ID is required')
    .isUUID()
    .withMessage('Device ID must be a valid UUID'),
  body('deviceName')
    .optional()
    .isLength({ min: 1, max: 100 })
    .withMessage('Device name must be between 1 and 100 characters'),
];

// Audit log query validation
export const auditLogValidation = [
  body('startDate')
    .optional()
    .isISO8601()
    .withMessage('Start date must be in ISO 8601 format'),
  body('endDate')
    .optional()
    .isISO8601()
    .withMessage('End date must be in ISO 8601 format'),
  body('userId')
    .optional()
    .isInt({ min: 1 })
    .withMessage('User ID must be a positive integer'),
  body('action')
    .optional()
    .isIn(['login', 'logout', 'register', 'password_change', 'role_change'])
    .withMessage('Invalid action type'),
];