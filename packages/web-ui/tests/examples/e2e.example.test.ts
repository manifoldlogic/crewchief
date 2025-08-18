/**
 * Example End-to-End Test
 * 
 * Demonstrates best practices for E2E testing using Playwright in the CrewChief Web UI project.
 * E2E tests verify complete user workflows from the browser perspective.
 */

import { test, expect, Page } from '@playwright/test';

// Test data setup
const testUser = {
  name: 'John Doe',
  email: 'john.doe@example.com'
};

const updatedUser = {
  name: 'John Smith',
  email: 'john.smith@example.com'
};

// Page Object Model example
class UserManagementPage {
  constructor(private page: Page) {}

  // Navigation
  async goto() {
    await this.page.goto('/users');
  }

  // User creation form
  async fillUserForm(user: { name: string; email: string }) {
    await this.page.fill('[data-testid="name-input"]', user.name);
    await this.page.fill('[data-testid="email-input"]', user.email);
  }

  async submitUserForm() {
    await this.page.click('[data-testid="submit-button"]');
  }

  async createUser(user: { name: string; email: string }) {
    await this.page.click('[data-testid="add-user-button"]');
    await this.fillUserForm(user);
    await this.submitUserForm();
  }

  // User list interactions
  async getUserRow(email: string) {
    return this.page.locator(`[data-testid="user-row"][data-email="${email}"]`);
  }

  async editUser(email: string) {
    const row = await this.getUserRow(email);
    await row.locator('[data-testid="edit-button"]').click();
  }

  async deleteUser(email: string) {
    const row = await this.getUserRow(email);
    await row.locator('[data-testid="delete-button"]').click();
    await this.page.click('[data-testid="confirm-delete"]');
  }

  // Search functionality
  async searchUsers(query: string) {
    await this.page.fill('[data-testid="search-input"]', query);
    await this.page.press('[data-testid="search-input"]', 'Enter');
  }

  async clearSearch() {
    await this.page.fill('[data-testid="search-input"]', '');
    await this.page.press('[data-testid="search-input"]', 'Enter');
  }

  // Assertions helpers
  async expectUserToBeVisible(user: { name: string; email: string }) {
    const row = await this.getUserRow(user.email);
    await expect(row).toBeVisible();
    await expect(row.locator('[data-testid="user-name"]')).toHaveText(user.name);
    await expect(row.locator('[data-testid="user-email"]')).toHaveText(user.email);
  }

  async expectUserNotToBeVisible(email: string) {
    const row = await this.getUserRow(email);
    await expect(row).not.toBeVisible();
  }

  async expectUsersCount(count: number) {
    await expect(this.page.locator('[data-testid="user-row"]')).toHaveCount(count);
  }

  async expectEmptyState() {
    await expect(this.page.locator('[data-testid="empty-state"]')).toBeVisible();
    await expect(this.page.locator('[data-testid="empty-state"]')).toContainText('No users found');
  }
}

test.describe('Example E2E Tests - User Management', () => {
  let userPage: UserManagementPage;

  test.beforeEach(async ({ page }) => {
    userPage = new UserManagementPage(page);
    await userPage.goto();
  });

  test.describe('User Creation', () => {
    test('creates a new user successfully', async ({ page }) => {
      // Initial state - no users
      await userPage.expectEmptyState();

      // Create new user
      await userPage.createUser(testUser);

      // Verify success message
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible();
      await expect(page.locator('[data-testid="success-message"]')).toContainText('User created successfully');

      // Verify user appears in list
      await userPage.expectUserToBeVisible(testUser);
      await userPage.expectUsersCount(1);
    });

    test('shows validation errors for invalid input', async ({ page }) => {
      await page.click('[data-testid="add-user-button"]');

      // Try to submit empty form
      await userPage.submitUserForm();

      // Check for validation errors
      await expect(page.locator('[data-testid="name-error"]')).toBeVisible();
      await expect(page.locator('[data-testid="name-error"]')).toContainText('Name is required');
      await expect(page.locator('[data-testid="email-error"]')).toBeVisible();
      await expect(page.locator('[data-testid="email-error"]')).toContainText('Email is required');

      // Fill name but invalid email
      await page.fill('[data-testid="name-input"]', 'John Doe');
      await page.fill('[data-testid="email-input"]', 'invalid-email');
      await userPage.submitUserForm();

      await expect(page.locator('[data-testid="email-error"]')).toContainText('Please enter a valid email');
    });

    test('prevents duplicate email addresses', async ({ page }) => {
      // Create first user
      await userPage.createUser(testUser);
      await userPage.expectUserToBeVisible(testUser);

      // Try to create user with same email
      await userPage.createUser({
        name: 'Jane Doe',
        email: testUser.email
      });

      // Verify error message
      await expect(page.locator('[data-testid="error-message"]')).toBeVisible();
      await expect(page.locator('[data-testid="error-message"]')).toContainText('User with this email already exists');

      // Verify only one user exists
      await userPage.expectUsersCount(1);
    });
  });

  test.describe('User Editing', () => {
    test.beforeEach(async () => {
      // Create a user to edit
      await userPage.createUser(testUser);
    });

    test('edits user information successfully', async ({ page }) => {
      await userPage.editUser(testUser.email);

      // Update form should be pre-filled
      await expect(page.locator('[data-testid="name-input"]')).toHaveValue(testUser.name);
      await expect(page.locator('[data-testid="email-input"]')).toHaveValue(testUser.email);

      // Update user information
      await userPage.fillUserForm(updatedUser);
      await userPage.submitUserForm();

      // Verify success message
      await expect(page.locator('[data-testid="success-message"]')).toContainText('User updated successfully');

      // Verify updated information
      await userPage.expectUserToBeVisible(updatedUser);
      await userPage.expectUserNotToBeVisible(testUser.email);
    });

    test('cancels edit operation', async ({ page }) => {
      await userPage.editUser(testUser.email);

      // Make changes but cancel
      await page.fill('[data-testid="name-input"]', 'Different Name');
      await page.click('[data-testid="cancel-button"]');

      // Verify original user data is preserved
      await userPage.expectUserToBeVisible(testUser);
    });
  });

  test.describe('User Deletion', () => {
    test.beforeEach(async () => {
      await userPage.createUser(testUser);
    });

    test('deletes user with confirmation', async ({ page }) => {
      await userPage.deleteUser(testUser.email);

      // Verify success message
      await expect(page.locator('[data-testid="success-message"]')).toContainText('User deleted successfully');

      // Verify user is removed
      await userPage.expectUserNotToBeVisible(testUser.email);
      await userPage.expectEmptyState();
    });

    test('cancels deletion', async ({ page }) => {
      const row = await userPage.getUserRow(testUser.email);
      await row.locator('[data-testid="delete-button"]').click();
      
      // Cancel deletion
      await page.click('[data-testid="cancel-delete"]');

      // Verify user still exists
      await userPage.expectUserToBeVisible(testUser);
    });
  });

  test.describe('Search Functionality', () => {
    test.beforeEach(async () => {
      // Create multiple users for search testing
      const users = [
        { name: 'John Doe', email: 'john.doe@example.com' },
        { name: 'Jane Smith', email: 'jane.smith@company.com' },
        { name: 'Bob Johnson', email: 'bob@example.org' },
        { name: 'Alice Brown', email: 'alice.brown@test.com' }
      ];

      for (const user of users) {
        await userPage.createUser(user);
      }
    });

    test('searches users by name', async () => {
      await userPage.searchUsers('john');

      // Should show John Doe and Bob Johnson (contains 'john')
      await userPage.expectUsersCount(2);
      await userPage.expectUserToBeVisible({ name: 'John Doe', email: 'john.doe@example.com' });
      await userPage.expectUserToBeVisible({ name: 'Bob Johnson', email: 'bob@example.org' });
    });

    test('searches users by email domain', async () => {
      await userPage.searchUsers('example.com');

      // Should show only John Doe
      await userPage.expectUsersCount(1);
      await userPage.expectUserToBeVisible({ name: 'John Doe', email: 'john.doe@example.com' });
    });

    test('shows no results for non-matching search', async () => {
      await userPage.searchUsers('nonexistent');

      await userPage.expectUsersCount(0);
      await userPage.expectEmptyState();
    });

    test('clears search to show all users', async () => {
      // First search for something
      await userPage.searchUsers('john');
      await userPage.expectUsersCount(2);

      // Clear search
      await userPage.clearSearch();

      // Should show all users again
      await userPage.expectUsersCount(4);
    });

    test('is case insensitive', async () => {
      await userPage.searchUsers('ALICE');

      await userPage.expectUsersCount(1);
      await userPage.expectUserToBeVisible({ name: 'Alice Brown', email: 'alice.brown@test.com' });
    });
  });

  test.describe('Responsive Design', () => {
    test('works correctly on mobile viewport', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 }); // iPhone SE

      await userPage.createUser(testUser);

      // Mobile layout adjustments
      await expect(page.locator('[data-testid="mobile-menu-button"]')).toBeVisible();
      await expect(page.locator('[data-testid="desktop-sidebar"]')).not.toBeVisible();

      // User row should adapt to mobile
      const userRow = await userPage.getUserRow(testUser.email);
      await expect(userRow.locator('[data-testid="mobile-user-card"]')).toBeVisible();
    });

    test('adapts to tablet viewport', async ({ page }) => {
      await page.setViewportSize({ width: 768, height: 1024 }); // iPad

      await userPage.createUser(testUser);

      // Tablet layout
      await expect(page.locator('[data-testid="tablet-layout"]')).toBeVisible();
      await userPage.expectUserToBeVisible(testUser);
    });
  });

  test.describe('Performance and Loading States', () => {
    test('shows loading state during operations', async ({ page }) => {
      // Intercept API calls to add delay
      await page.route('**/api/users', async route => {
        await new Promise(resolve => setTimeout(resolve, 1000));
        route.continue();
      });

      const createPromise = userPage.createUser(testUser);

      // Check loading indicator appears
      await expect(page.locator('[data-testid="loading-spinner"]')).toBeVisible();

      await createPromise;

      // Loading indicator should disappear
      await expect(page.locator('[data-testid="loading-spinner"]')).not.toBeVisible();
    });

    test('handles slow network gracefully', async ({ page }) => {
      // Simulate slow network
      await page.route('**/api/users', async route => {
        await new Promise(resolve => setTimeout(resolve, 3000));
        route.continue();
      });

      await page.click('[data-testid="add-user-button"]');
      await userPage.fillUserForm(testUser);
      
      const submitPromise = userPage.submitUserForm();

      // Button should be disabled during submission
      await expect(page.locator('[data-testid="submit-button"]')).toBeDisabled();
      await expect(page.locator('[data-testid="submit-button"]')).toContainText('Creating...');

      await submitPromise;

      // Button should be enabled again
      await expect(page.locator('[data-testid="submit-button"]')).not.toBeDisabled();
    });
  });

  test.describe('Error Handling', () => {
    test('handles API errors gracefully', async ({ page }) => {
      // Mock API to return error
      await page.route('**/api/users', route => {
        route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Internal server error' })
        });
      });

      await userPage.createUser(testUser);

      // Should show error message
      await expect(page.locator('[data-testid="error-message"]')).toBeVisible();
      await expect(page.locator('[data-testid="error-message"]')).toContainText('Failed to create user');
    });

    test('handles network failures', async ({ page }) => {
      // Simulate network failure
      await page.route('**/api/users', route => {
        route.abort('failed');
      });

      await userPage.createUser(testUser);

      // Should show network error message
      await expect(page.locator('[data-testid="error-message"]')).toContainText('Network error');
    });
  });

  test.describe('Accessibility', () => {
    test('supports keyboard navigation', async ({ page }) => {
      await userPage.createUser(testUser);

      // Tab through interactive elements
      await page.keyboard.press('Tab'); // Focus on first interactive element
      await page.keyboard.press('Tab'); // Add user button
      await page.keyboard.press('Enter'); // Activate add user

      // Form should open
      await expect(page.locator('[data-testid="user-form"]')).toBeVisible();

      // Tab through form fields
      await page.keyboard.press('Tab'); // Name input
      await page.keyboard.type('Test User');
      await page.keyboard.press('Tab'); // Email input
      await page.keyboard.type('test@example.com');
      await page.keyboard.press('Tab'); // Submit button
      await page.keyboard.press('Enter'); // Submit
    });

    test('has proper ARIA labels and roles', async ({ page }) => {
      await userPage.createUser(testUser);

      // Check ARIA attributes
      await expect(page.locator('[data-testid="user-list"]')).toHaveAttribute('role', 'table');
      await expect(page.locator('[data-testid="add-user-button"]')).toHaveAttribute('aria-label', 'Add new user');
      await expect(page.locator('[data-testid="search-input"]')).toHaveAttribute('aria-label', 'Search users');
    });

    test('supports screen readers', async ({ page }) => {
      await userPage.createUser(testUser);

      // Check for screen reader content
      await expect(page.locator('[data-testid="user-count"]')).toContainText('1 user total');
      
      const userRow = await userPage.getUserRow(testUser.email);
      await expect(userRow).toHaveAttribute('aria-label', `User: ${testUser.name}, Email: ${testUser.email}`);
    });
  });
});