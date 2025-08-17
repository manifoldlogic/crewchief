import { test, expect } from '@playwright/test';

test.describe('Dashboard Page', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');
    
    // Wait for the page to load
    await page.waitForLoadState('networkidle');
  });

  test('should display dashboard title', async ({ page }) => {
    // Check that the page title is correct
    await expect(page).toHaveTitle(/CrewChief/);
    
    // Check that the dashboard heading is visible
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('should have responsive navigation', async ({ page }) => {
    // Check if sidebar is visible on desktop
    const sidebar = page.getByTestId('sidebar');
    
    // On desktop, sidebar should be visible
    await expect(sidebar).toBeVisible();
    
    // Test mobile view
    await page.setViewportSize({ width: 375, height: 667 });
    
    // Mobile menu button should be visible
    const mobileMenuButton = page.getByRole('button', { name: /open sidebar/i });
    await expect(mobileMenuButton).toBeVisible();
    
    // Sidebar should be hidden on mobile by default
    await expect(sidebar).toBeHidden();
    
    // Click mobile menu to open sidebar
    await mobileMenuButton.click();
    await expect(sidebar).toBeVisible();
  });

  test('should display status indicators', async ({ page }) => {
    // Check for online status indicator
    await expect(page.getByText('Online')).toBeVisible();
    
    // Check for status dot
    const statusIndicator = page.locator('.bg-green-400');
    await expect(statusIndicator).toBeVisible();
  });

  test('should navigate between pages', async ({ page }) => {
    // Navigate to Search page
    await page.getByRole('link', { name: /search/i }).click();
    await expect(page.getByRole('heading', { name: 'Search' })).toBeVisible();
    
    // Navigate to Worktrees page
    await page.getByRole('link', { name: /worktrees/i }).click();
    await expect(page.getByRole('heading', { name: 'Worktrees' })).toBeVisible();
    
    // Navigate to Agents page
    await page.getByRole('link', { name: /agents/i }).click();
    await expect(page.getByRole('heading', { name: 'Agents' })).toBeVisible();
    
    // Navigate back to Dashboard
    await page.getByRole('link', { name: /dashboard/i }).click();
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('should maintain state during navigation', async ({ page }) => {
    // Start on dashboard
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
    
    // Navigate away and back
    await page.getByRole('link', { name: /search/i }).click();
    await page.goBack();
    
    // Should still be on dashboard
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('should be accessible', async ({ page }) => {
    // Check that main elements have proper ARIA labels
    await expect(page.getByRole('banner')).toBeVisible(); // Header
    await expect(page.getByRole('navigation')).toBeVisible(); // Sidebar
    await expect(page.getByRole('main')).toBeVisible(); // Main content
    
    // Check that interactive elements are keyboard accessible
    const menuButton = page.getByRole('button', { name: /open sidebar/i });
    await menuButton.focus();
    await expect(menuButton).toBeFocused();
    
    // Tab through navigation links
    const firstNavLink = page.getByRole('link', { name: /dashboard/i }).first();
    await firstNavLink.focus();
    await expect(firstNavLink).toBeFocused();
  });

  test('should handle errors gracefully', async ({ page }) => {
    // Mock a failed API request
    await page.route('**/api/**', route => {
      route.abort('failed');
    });
    
    // Reload the page to trigger error state
    await page.reload();
    
    // The app should still load basic UI even if API calls fail
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('should support keyboard navigation', async ({ page }) => {
    // Tab through the interface
    await page.keyboard.press('Tab'); // Should focus first interactive element
    
    // Use arrow keys to navigate if applicable
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowUp');
    
    // Enter should activate focused element
    await page.keyboard.press('Enter');
  });
});