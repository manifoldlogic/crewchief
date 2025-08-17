import { test, expect } from '@playwright/test';

test.describe('Search Functionality', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the search page
    await page.goto('/search');
    await page.waitForLoadState('networkidle');
  });

  test('should display search interface', async ({ page }) => {
    // Check page title
    await expect(page.getByRole('heading', { name: 'Search' })).toBeVisible();
    
    // Check search input is present
    const searchInput = page.getByRole('textbox', { name: /search/i });
    await expect(searchInput).toBeVisible();
    
    // Check search button is present
    const searchButton = page.getByRole('button', { name: /search/i });
    await expect(searchButton).toBeVisible();
  });

  test('should perform a basic search', async ({ page }) => {
    // Mock search API response
    await page.route('**/api/maproom/search', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          query: 'test function',
          results: [
            {
              id: 'test-result-1',
              file_path: '/src/test.ts',
              line_start: 1,
              line_end: 10,
              content: 'function test() { return true; }',
              relevance_score: 0.95,
              language: 'typescript',
            },
            {
              id: 'test-result-2',
              file_path: '/src/utils.ts',
              line_start: 5,
              line_end: 15,
              content: 'export function testUtils() { /* ... */ }',
              relevance_score: 0.85,
              language: 'typescript',
            },
          ],
          totalCount: 2,
          executionTimeMs: 150,
          filters: {},
          cached: false,
        }),
      });
    });

    // Enter search query
    const searchInput = page.getByRole('textbox', { name: /search/i });
    await searchInput.fill('test function');
    
    // Submit search
    const searchButton = page.getByRole('button', { name: /search/i });
    await searchButton.click();
    
    // Wait for results to load
    await page.waitForSelector('[data-testid="search-results"]', { timeout: 5000 });
    
    // Check that results are displayed
    await expect(page.getByText('/src/test.ts')).toBeVisible();
    await expect(page.getByText('/src/utils.ts')).toBeVisible();
    await expect(page.getByText('function test() { return true; }')).toBeVisible();
    
    // Check result count
    await expect(page.getByText('2 results')).toBeVisible();
    
    // Check execution time is displayed
    await expect(page.getByText(/150ms/)).toBeVisible();
  });

  test('should handle empty search results', async ({ page }) => {
    // Mock empty search response
    await page.route('**/api/maproom/search', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          query: 'nonexistent',
          results: [],
          totalCount: 0,
          executionTimeMs: 50,
          filters: {},
          cached: false,
        }),
      });
    });

    // Perform search
    await page.getByRole('textbox', { name: /search/i }).fill('nonexistent');
    await page.getByRole('button', { name: /search/i }).click();
    
    // Wait for response
    await page.waitForTimeout(1000);
    
    // Check for empty state message
    await expect(page.getByText(/no results found/i)).toBeVisible();
    await expect(page.getByText('0 results')).toBeVisible();
  });

  test('should apply search filters', async ({ page }) => {
    // Mock filtered search response
    await page.route('**/api/maproom/search', route => {
      const request = route.request();
      const postData = JSON.parse(request.postData() || '{}');
      
      expect(postData.filters).toMatchObject({
        language: 'typescript',
        maxResults: 10,
      });
      
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          query: 'function',
          results: [],
          totalCount: 0,
          executionTimeMs: 100,
          filters: postData.filters,
          cached: false,
        }),
      });
    });

    // Open filter options
    const filterToggle = page.getByRole('button', { name: /filters/i });
    await filterToggle.click();
    
    // Select language filter
    const languageSelect = page.getByRole('combobox', { name: /language/i });
    await languageSelect.selectOption('typescript');
    
    // Set max results
    const maxResultsInput = page.getByRole('spinbutton', { name: /max results/i });
    await maxResultsInput.fill('10');
    
    // Perform search with filters
    await page.getByRole('textbox', { name: /search/i }).fill('function');
    await page.getByRole('button', { name: /search/i }).click();
    
    // Verify filters are applied (checked in route mock above)
  });

  test('should handle search errors', async ({ page }) => {
    // Mock error response
    await page.route('**/api/maproom/search', route => {
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'Search service unavailable',
        }),
      });
    });

    // Perform search
    await page.getByRole('textbox', { name: /search/i }).fill('test');
    await page.getByRole('button', { name: /search/i }).click();
    
    // Check for error message
    await expect(page.getByText(/search service unavailable/i)).toBeVisible();
  });

  test('should support search keyboard shortcuts', async ({ page }) => {
    const searchInput = page.getByRole('textbox', { name: /search/i });
    
    // Focus search input with Ctrl+K or Cmd+K
    await page.keyboard.press(process.platform === 'darwin' ? 'Meta+k' : 'Control+k');
    await expect(searchInput).toBeFocused();
    
    // Type and submit with Enter
    await searchInput.fill('test');
    await page.keyboard.press('Enter');
    
    // Should trigger search
    await page.waitForTimeout(500);
  });

  test('should show search history', async ({ page }) => {
    // Mock search history API
    await page.route('**/api/search/history', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { query: 'previous search 1', timestamp: '2024-01-01T00:00:00Z' },
          { query: 'previous search 2', timestamp: '2024-01-01T00:01:00Z' },
        ]),
      });
    });

    // Open search history
    const historyButton = page.getByRole('button', { name: /history/i });
    await historyButton.click();
    
    // Check history items are displayed
    await expect(page.getByText('previous search 1')).toBeVisible();
    await expect(page.getByText('previous search 2')).toBeVisible();
  });

  test('should allow result interaction', async ({ page }) => {
    // Mock search response with results
    await page.route('**/api/maproom/search', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          query: 'test',
          results: [
            {
              id: 'result-1',
              file_path: '/src/test.ts',
              line_start: 1,
              line_end: 10,
              content: 'function test() { return true; }',
              relevance_score: 0.95,
              language: 'typescript',
            },
          ],
          totalCount: 1,
          executionTimeMs: 100,
          filters: {},
          cached: false,
        }),
      });
    });

    // Perform search
    await page.getByRole('textbox', { name: /search/i }).fill('test');
    await page.getByRole('button', { name: /search/i }).click();
    
    // Wait for results
    await page.waitForSelector('[data-testid="search-result"]');
    
    // Click on a result
    const firstResult = page.getByTestId('search-result').first();
    await firstResult.click();
    
    // Should show more details or navigate to file
    // (Exact behavior depends on implementation)
  });

  test('should be responsive on mobile', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    
    // Search interface should be responsive
    const searchInput = page.getByRole('textbox', { name: /search/i });
    await expect(searchInput).toBeVisible();
    
    // Filters might be collapsed on mobile
    const mobileFiltersButton = page.getByRole('button', { name: /filters/i });
    if (await mobileFiltersButton.isVisible()) {
      await mobileFiltersButton.click();
    }
  });

  test('should preserve search state on navigation', async ({ page }) => {
    // Perform search
    await page.getByRole('textbox', { name: /search/i }).fill('persistent search');
    await page.getByRole('button', { name: /search/i }).click();
    
    // Navigate away
    await page.getByRole('link', { name: /dashboard/i }).click();
    
    // Navigate back
    await page.getByRole('link', { name: /search/i }).click();
    
    // Search query should be preserved
    const searchInput = page.getByRole('textbox', { name: /search/i });
    await expect(searchInput).toHaveValue('persistent search');
  });
});