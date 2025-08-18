/**
 * Accessibility utilities for WCAG 2.1 AA compliance
 */

/**
 * Generates a unique ID for form elements and ARIA attributes
 */
export function generateId(prefix: string = 'cc'): string {
  return `${prefix}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Checks if an element has sufficient color contrast for WCAG AA compliance
 * Note: This is a simplified check. For production, use a proper color contrast library
 */
export function hasGoodContrast(foreground: string, background: string): boolean {
  // Simplified implementation - in practice, you'd use a proper color contrast calculation
  // This is a placeholder that always returns true for now
  // Consider using libraries like 'color-contrast' or 'wcag-contrast' for real implementations
  return true;
}

/**
 * Creates screen reader only text for complex UI elements
 */
export function createScreenReaderText(text: string): React.CSSProperties {
  return {
    position: 'absolute',
    width: '1px',
    height: '1px',
    padding: 0,
    margin: '-1px',
    overflow: 'hidden',
    clip: 'rect(0, 0, 0, 0)',
    whiteSpace: 'nowrap',
    border: 0,
  };
}

/**
 * ARIA attributes for common UI patterns
 */
export const ariaAttributes = {
  // Button states
  button: {
    pressed: (pressed: boolean) => ({ 'aria-pressed': pressed }),
    expanded: (expanded: boolean) => ({ 'aria-expanded': expanded }),
    disabled: (disabled: boolean) => ({ 'aria-disabled': disabled }),
  },
  
  // Form field states
  field: {
    required: { 'aria-required': true },
    invalid: (invalid: boolean) => ({ 'aria-invalid': invalid }),
    describedBy: (ids: string[]) => ({ 'aria-describedby': ids.join(' ') }),
    labelledBy: (ids: string[]) => ({ 'aria-labelledby': ids.join(' ') }),
  },
  
  // Dialog/Modal states
  dialog: {
    modal: { 'aria-modal': true },
    labelledBy: (id: string) => ({ 'aria-labelledby': id }),
    describedBy: (id: string) => ({ 'aria-describedby': id }),
  },
  
  // Loading states
  loading: {
    busy: { 'aria-busy': true },
    live: (politeness: 'polite' | 'assertive' = 'polite') => ({ 'aria-live': politeness }),
  },
};

/**
 * Keyboard navigation helpers
 */
export const keyboardNavigation = {
  // Common key codes
  keys: {
    ENTER: 'Enter',
    SPACE: ' ',
    ESCAPE: 'Escape',
    ARROW_UP: 'ArrowUp',
    ARROW_DOWN: 'ArrowDown',
    ARROW_LEFT: 'ArrowLeft',
    ARROW_RIGHT: 'ArrowRight',
    TAB: 'Tab',
    HOME: 'Home',
    END: 'End',
  },
  
  // Focus management
  trapFocus: (container: HTMLElement) => {
    const focusableElements = container.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const firstElement = focusableElements[0] as HTMLElement;
    const lastElement = focusableElements[focusableElements.length - 1] as HTMLElement;
    
    const handleTab = (e: KeyboardEvent) => {
      if (e.key === 'Tab') {
        if (e.shiftKey) {
          if (document.activeElement === firstElement) {
            lastElement?.focus();
            e.preventDefault();
          }
        } else {
          if (document.activeElement === lastElement) {
            firstElement?.focus();
            e.preventDefault();
          }
        }
      }
    };
    
    container.addEventListener('keydown', handleTab);
    firstElement?.focus();
    
    return () => {
      container.removeEventListener('keydown', handleTab);
    };
  },
};

/**
 * Announce messages to screen readers
 */
export function announceToScreenReader(message: string, priority: 'polite' | 'assertive' = 'polite') {
  const announcement = document.createElement('div');
  announcement.setAttribute('aria-live', priority);
  announcement.setAttribute('aria-atomic', 'true');
  announcement.style.cssText = Object.entries(createScreenReaderText(message))
    .map(([key, value]) => `${key}: ${value}`)
    .join('; ');
  
  document.body.appendChild(announcement);
  announcement.textContent = message;
  
  // Clean up after announcement
  setTimeout(() => {
    document.body.removeChild(announcement);
  }, 1000);
}

/**
 * Focus management utilities
 */
export const focusManagement = {
  // Get all focusable elements within a container
  getFocusableElements: (container: HTMLElement): HTMLElement[] => {
    const selector = [
      'button:not([disabled])',
      '[href]',
      'input:not([disabled])',
      'select:not([disabled])',
      'textarea:not([disabled])',
      '[tabindex]:not([tabindex="-1"]):not([disabled])',
      '[contenteditable="true"]',
    ].join(', ');
    
    return Array.from(container.querySelectorAll(selector));
  },
  
  // Save and restore focus
  saveFocus: (): (() => void) => {
    const previouslyFocused = document.activeElement as HTMLElement;
    return () => {
      previouslyFocused?.focus();
    };
  },
  
  // Focus first element in container
  focusFirst: (container: HTMLElement): boolean => {
    const focusableElements = focusManagement.getFocusableElements(container);
    if (focusableElements.length > 0) {
      focusableElements[0].focus();
      return true;
    }
    return false;
  },
};

/**
 * Reduced motion detection
 */
export function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/**
 * High contrast mode detection
 */
export function prefersHighContrast(): boolean {
  return window.matchMedia('(prefers-contrast: high)').matches;
}

/**
 * Validate ARIA attributes
 */
export function validateAriaAttributes(element: HTMLElement): string[] {
  const warnings: string[] = [];
  
  // Check for required aria-label or aria-labelledby on interactive elements
  const interactiveRoles = ['button', 'link', 'menuitem', 'tab', 'checkbox', 'radio'];
  const role = element.getAttribute('role') || element.tagName.toLowerCase();
  
  if (interactiveRoles.includes(role)) {
    const hasLabel = element.getAttribute('aria-label') || 
                    element.getAttribute('aria-labelledby') ||
                    element.textContent?.trim();
    
    if (!hasLabel) {
      warnings.push(`Interactive element lacks accessible name`);
    }
  }
  
  // Check for proper heading hierarchy
  if (/^h[1-6]$/i.test(element.tagName)) {
    const level = parseInt(element.tagName.charAt(1));
    const previousHeading = element.previousElementSibling?.closest('h1, h2, h3, h4, h5, h6');
    if (previousHeading) {
      const previousLevel = parseInt(previousHeading.tagName.charAt(1));
      if (level > previousLevel + 1) {
        warnings.push(`Heading level skipped from h${previousLevel} to h${level}`);
      }
    }
  }
  
  return warnings;
}