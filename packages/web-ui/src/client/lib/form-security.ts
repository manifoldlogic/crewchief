import DOMPurify from "dompurify";

/**
 * Sanitizes user input to prevent XSS attacks
 */
export function sanitizeInput(input: string): string {
  return DOMPurify.sanitize(input, { 
    ALLOWED_TAGS: [],
    ALLOWED_ATTR: [],
    KEEP_CONTENT: true 
  });
}

/**
 * Sanitizes an object's string values recursively
 */
export function sanitizeFormData<T extends Record<string, any>>(data: T): T {
  const sanitized = { ...data };
  
  for (const key in sanitized) {
    const value = sanitized[key];
    if (typeof value === 'string') {
      sanitized[key] = sanitizeInput(value);
    } else if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      sanitized[key] = sanitizeFormData(value);
    } else if (Array.isArray(value)) {
      sanitized[key] = value.map(item => 
        typeof item === 'string' ? sanitizeInput(item) :
        typeof item === 'object' && item !== null ? sanitizeFormData(item) :
        item
      );
    }
  }
  
  return sanitized;
}

/**
 * Gets CSRF token from meta tag or cookie
 */
export function getCSRFToken(): string | null {
  // Try to get from meta tag first
  const metaTag = document.querySelector('meta[name="csrf-token"]') as HTMLMetaElement;
  if (metaTag?.content) {
    return metaTag.content;
  }
  
  // Fallback to cookie
  const match = document.cookie.match(/csrf_token=([^;]+)/);
  return match ? decodeURIComponent(match[1]) : null;
}

/**
 * Adds CSRF token to form data
 */
export function addCSRFToken<T extends Record<string, any>>(data: T): T & { _token?: string } {
  const token = getCSRFToken();
  return token ? { ...data, _token: token } : data;
}

/**
 * Validates that required fields are present and not empty
 */
export function validateRequiredFields<T extends Record<string, any>>(
  data: T,
  requiredFields: (keyof T)[]
): { isValid: boolean; missingFields: string[] } {
  const missingFields: string[] = [];
  
  for (const field of requiredFields) {
    const value = data[field];
    if (value === undefined || value === null || value === '') {
      missingFields.push(String(field));
    }
  }
  
  return {
    isValid: missingFields.length === 0,
    missingFields
  };
}

/**
 * Rate limiting for form submissions
 */
class FormRateLimit {
  private submissions = new Map<string, number[]>();
  private readonly maxSubmissions: number;
  private readonly timeWindow: number; // in milliseconds

  constructor(maxSubmissions = 5, timeWindowMinutes = 1) {
    this.maxSubmissions = maxSubmissions;
    this.timeWindow = timeWindowMinutes * 60 * 1000;
  }

  canSubmit(formId: string): boolean {
    const now = Date.now();
    const formSubmissions = this.submissions.get(formId) || [];
    
    // Remove old submissions outside time window
    const recentSubmissions = formSubmissions.filter(
      timestamp => now - timestamp < this.timeWindow
    );
    
    this.submissions.set(formId, recentSubmissions);
    
    return recentSubmissions.length < this.maxSubmissions;
  }

  recordSubmission(formId: string): void {
    const now = Date.now();
    const formSubmissions = this.submissions.get(formId) || [];
    formSubmissions.push(now);
    this.submissions.set(formId, formSubmissions);
  }

  getRemainingAttempts(formId: string): number {
    const now = Date.now();
    const formSubmissions = this.submissions.get(formId) || [];
    const recentSubmissions = formSubmissions.filter(
      timestamp => now - timestamp < this.timeWindow
    );
    
    return Math.max(0, this.maxSubmissions - recentSubmissions.length);
  }

  getTimeUntilReset(formId: string): number {
    const now = Date.now();
    const formSubmissions = this.submissions.get(formId) || [];
    
    if (formSubmissions.length === 0) return 0;
    
    const oldestRecentSubmission = Math.min(...formSubmissions.filter(
      timestamp => now - timestamp < this.timeWindow
    ));
    
    return Math.max(0, this.timeWindow - (now - oldestRecentSubmission));
  }
}

export const defaultRateLimit = new FormRateLimit();

/**
 * Secure form submission helper
 */
export interface SecureFormOptions {
  sanitize?: boolean;
  addCSRF?: boolean;
  rateLimit?: boolean;
  formId?: string;
  requiredFields?: string[];
}

export function prepareSecureFormData<T extends Record<string, any>>(
  data: T,
  options: SecureFormOptions = {}
): { data: T; isValid: boolean; errors: string[] } {
  const {
    sanitize = true,
    addCSRF = true,
    rateLimit = true,
    formId = 'default',
    requiredFields = []
  } = options;

  const errors: string[] = [];
  let processedData = { ...data };

  // Validate required fields
  if (requiredFields.length > 0) {
    const validation = validateRequiredFields(processedData, requiredFields);
    if (!validation.isValid) {
      errors.push(`Missing required fields: ${validation.missingFields.join(', ')}`);
    }
  }

  // Check rate limiting
  if (rateLimit && !defaultRateLimit.canSubmit(formId)) {
    const timeUntilReset = Math.ceil(defaultRateLimit.getTimeUntilReset(formId) / 1000);
    errors.push(`Too many submissions. Please wait ${timeUntilReset} seconds before trying again.`);
  }

  // Sanitize input
  if (sanitize) {
    processedData = sanitizeFormData(processedData);
  }

  // Add CSRF token
  if (addCSRF) {
    processedData = addCSRFToken(processedData);
  }

  // Record submission for rate limiting
  if (rateLimit && errors.length === 0) {
    defaultRateLimit.recordSubmission(formId);
  }

  return {
    data: processedData,
    isValid: errors.length === 0,
    errors
  };
}