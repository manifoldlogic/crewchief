//! Token budget management for context assembly.
//!
//! This module provides the core budget management infrastructure that ensures
//! assembled context never exceeds specified token limits while maximizing
//! the value of included content through intelligent allocation and tracking.

use std::collections::HashMap;

/// Budget allocation across different context categories.
///
/// Allocates tokens based on importance and typical needs:
/// - Primary: 40% - The main chunk being explained
/// - Tests: 20% - Test coverage for understanding usage
/// - Callers: 15% - Functions that call the primary chunk
/// - Callees: 15% - Functions called by the primary chunk
/// - Config: 10% - Configuration and metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetAllocation {
    /// Tokens allocated for the primary chunk (40%)
    pub primary: usize,
    /// Tokens allocated for test chunks (20%)
    pub tests: usize,
    /// Tokens allocated for caller chunks (15%)
    pub callers: usize,
    /// Tokens allocated for callee chunks (15%)
    pub callees: usize,
    /// Tokens allocated for config chunks (10%)
    pub config: usize,
}

impl BudgetAllocation {
    /// Calculate the total allocated tokens.
    pub fn total(&self) -> usize {
        self.primary + self.tests + self.callers + self.callees + self.config
    }
}

/// Statistics about budget usage across categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageStats {
    /// Total budget available
    pub budget: usize,
    /// Total tokens used so far
    pub used: usize,
    /// Tokens remaining
    pub remaining: usize,
    /// Usage breakdown by category
    pub by_category: HashMap<String, usize>,
}

/// Manages token budgets for context assembly with reservation and allocation.
///
/// The budget manager tracks token usage across different categories (primary,
/// tests, callers, callees, config) and ensures the total never exceeds the
/// specified budget. It supports:
/// - Reserving tokens before content is loaded
/// - Releasing reserved tokens if content isn't used
/// - Allocating budget percentages across categories
/// - Tracking actual usage vs. allocated budget
///
/// # Example
///
/// ```
/// use crewchief_maproom::context::budget::TokenBudgetManager;
///
/// let mut manager = TokenBudgetManager::new(8000);
///
/// // Reserve tokens for primary chunk
/// assert!(manager.reserve("primary", 3000));
///
/// // Check remaining budget
/// assert_eq!(manager.remaining(), 5000);
///
/// // Get allocation breakdown
/// let allocation = manager.allocate();
/// assert_eq!(allocation.primary, 3200); // 40% of 8000
/// ```
#[derive(Debug, Clone)]
pub struct TokenBudgetManager {
    /// Total token budget
    budget: usize,
    /// Tokens currently used or reserved
    used: usize,
    /// Reserved tokens by category name
    reserved: HashMap<String, usize>,
}

impl TokenBudgetManager {
    /// Create a new budget manager with the specified token budget.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::budget::TokenBudgetManager;
    ///
    /// let manager = TokenBudgetManager::new(8000);
    /// assert_eq!(manager.remaining(), 8000);
    /// ```
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            used: 0,
            reserved: HashMap::new(),
        }
    }

    /// Reserve tokens for a specific category.
    ///
    /// Returns `true` if the reservation succeeds (enough budget remaining),
    /// `false` if it would exceed the budget.
    ///
    /// If a category already has a reservation, this replaces it and adjusts
    /// the used count accordingly.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::budget::TokenBudgetManager;
    ///
    /// let mut manager = TokenBudgetManager::new(1000);
    ///
    /// // Reserve 400 tokens for primary
    /// assert!(manager.reserve("primary", 400));
    /// assert_eq!(manager.remaining(), 600);
    ///
    /// // Reserve more for tests
    /// assert!(manager.reserve("tests", 300));
    /// assert_eq!(manager.remaining(), 300);
    ///
    /// // This would exceed budget
    /// assert!(!manager.reserve("callers", 400));
    /// ```
    pub fn reserve(&mut self, category: &str, tokens: usize) -> bool {
        // Calculate what the new used total would be
        let existing = self.reserved.get(category).copied().unwrap_or(0);
        let new_used = self.used - existing + tokens;

        // Check if this would exceed budget
        if new_used > self.budget {
            return false;
        }

        // Update reservation
        self.reserved.insert(category.to_string(), tokens);
        self.used = new_used;
        true
    }

    /// Release a category's reservation, freeing up tokens.
    ///
    /// This is useful when content won't be included after all (e.g., file
    /// loading failed or priority cutoff).
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::budget::TokenBudgetManager;
    ///
    /// let mut manager = TokenBudgetManager::new(1000);
    ///
    /// manager.reserve("primary", 400);
    /// assert_eq!(manager.remaining(), 600);
    ///
    /// manager.release("primary");
    /// assert_eq!(manager.remaining(), 1000);
    /// ```
    pub fn release(&mut self, category: &str) {
        if let Some(tokens) = self.reserved.remove(category) {
            self.used -= tokens;
        }
    }

    /// Calculate budget allocation percentages across categories.
    ///
    /// Returns a `BudgetAllocation` with tokens allocated as:
    /// - Primary: 40%
    /// - Tests: 20%
    /// - Callers: 15%
    /// - Callees: 15%
    /// - Config: 10%
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::budget::TokenBudgetManager;
    ///
    /// let manager = TokenBudgetManager::new(10000);
    /// let allocation = manager.allocate();
    ///
    /// assert_eq!(allocation.primary, 4000); // 40%
    /// assert_eq!(allocation.tests, 2000);   // 20%
    /// assert_eq!(allocation.callers, 1500); // 15%
    /// assert_eq!(allocation.callees, 1500); // 15%
    /// assert_eq!(allocation.config, 1000);  // 10%
    /// ```
    pub fn allocate(&self) -> BudgetAllocation {
        BudgetAllocation {
            primary: (self.budget * 40) / 100,
            tests: (self.budget * 20) / 100,
            callers: (self.budget * 15) / 100,
            callees: (self.budget * 15) / 100,
            config: (self.budget * 10) / 100,
        }
    }

    /// Get the remaining token budget.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::budget::TokenBudgetManager;
    ///
    /// let mut manager = TokenBudgetManager::new(1000);
    /// assert_eq!(manager.remaining(), 1000);
    ///
    /// manager.reserve("primary", 300);
    /// assert_eq!(manager.remaining(), 700);
    /// ```
    pub fn remaining(&self) -> usize {
        self.budget.saturating_sub(self.used)
    }

    /// Get the total budget.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::budget::TokenBudgetManager;
    ///
    /// let manager = TokenBudgetManager::new(8000);
    /// assert_eq!(manager.budget(), 8000);
    /// ```
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Get the currently used token count.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::budget::TokenBudgetManager;
    ///
    /// let mut manager = TokenBudgetManager::new(1000);
    /// assert_eq!(manager.used(), 0);
    ///
    /// manager.reserve("primary", 300);
    /// assert_eq!(manager.used(), 300);
    /// ```
    pub fn used(&self) -> usize {
        self.used
    }

    /// Get detailed usage statistics.
    ///
    /// Returns statistics including total budget, used tokens, remaining tokens,
    /// and a breakdown of usage by category.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::budget::TokenBudgetManager;
    ///
    /// let mut manager = TokenBudgetManager::new(1000);
    /// manager.reserve("primary", 400);
    /// manager.reserve("tests", 200);
    ///
    /// let stats = manager.usage_stats();
    /// assert_eq!(stats.budget, 1000);
    /// assert_eq!(stats.used, 600);
    /// assert_eq!(stats.remaining, 400);
    /// assert_eq!(stats.by_category.get("primary"), Some(&400));
    /// assert_eq!(stats.by_category.get("tests"), Some(&200));
    /// ```
    pub fn usage_stats(&self) -> UsageStats {
        UsageStats {
            budget: self.budget,
            used: self.used,
            remaining: self.remaining(),
            by_category: self.reserved.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_budget_manager() {
        let manager = TokenBudgetManager::new(8000);
        assert_eq!(manager.budget(), 8000);
        assert_eq!(manager.used(), 0);
        assert_eq!(manager.remaining(), 8000);
    }

    #[test]
    fn test_reserve_within_budget() {
        let mut manager = TokenBudgetManager::new(1000);

        assert!(manager.reserve("primary", 400));
        assert_eq!(manager.used(), 400);
        assert_eq!(manager.remaining(), 600);

        assert!(manager.reserve("tests", 300));
        assert_eq!(manager.used(), 700);
        assert_eq!(manager.remaining(), 300);
    }

    #[test]
    fn test_reserve_exceeds_budget() {
        let mut manager = TokenBudgetManager::new(1000);

        manager.reserve("primary", 600);
        // This would exceed budget
        assert!(!manager.reserve("tests", 500));
        // Original reservation unchanged
        assert_eq!(manager.used(), 600);
    }

    #[test]
    fn test_reserve_replaces_existing() {
        let mut manager = TokenBudgetManager::new(1000);

        manager.reserve("primary", 300);
        assert_eq!(manager.used(), 300);

        // Replace with larger reservation
        manager.reserve("primary", 500);
        assert_eq!(manager.used(), 500);

        // Replace with smaller reservation
        manager.reserve("primary", 200);
        assert_eq!(manager.used(), 200);
    }

    #[test]
    fn test_reserve_exact_budget() {
        let mut manager = TokenBudgetManager::new(1000);

        assert!(manager.reserve("primary", 1000));
        assert_eq!(manager.remaining(), 0);

        // Can't reserve more
        assert!(!manager.reserve("tests", 1));
    }

    #[test]
    fn test_release_reservation() {
        let mut manager = TokenBudgetManager::new(1000);

        manager.reserve("primary", 400);
        manager.reserve("tests", 300);
        assert_eq!(manager.used(), 700);

        manager.release("primary");
        assert_eq!(manager.used(), 300);
        assert_eq!(manager.remaining(), 700);

        manager.release("tests");
        assert_eq!(manager.used(), 0);
        assert_eq!(manager.remaining(), 1000);
    }

    #[test]
    fn test_release_nonexistent_category() {
        let mut manager = TokenBudgetManager::new(1000);

        manager.reserve("primary", 400);
        manager.release("tests"); // Doesn't exist
        assert_eq!(manager.used(), 400); // Unchanged
    }

    #[test]
    fn test_allocate_percentages() {
        let manager = TokenBudgetManager::new(10000);
        let allocation = manager.allocate();

        assert_eq!(allocation.primary, 4000); // 40%
        assert_eq!(allocation.tests, 2000);   // 20%
        assert_eq!(allocation.callers, 1500); // 15%
        assert_eq!(allocation.callees, 1500); // 15%
        assert_eq!(allocation.config, 1000);  // 10%
        assert_eq!(allocation.total(), 10000);
    }

    #[test]
    fn test_allocate_rounding() {
        let manager = TokenBudgetManager::new(1000);
        let allocation = manager.allocate();

        // Check rounding behavior
        assert_eq!(allocation.primary, 400); // 40% of 1000
        assert_eq!(allocation.tests, 200);   // 20% of 1000
        assert_eq!(allocation.callers, 150); // 15% of 1000
        assert_eq!(allocation.callees, 150); // 15% of 1000
        assert_eq!(allocation.config, 100);  // 10% of 1000
    }

    #[test]
    fn test_allocate_small_budget() {
        let manager = TokenBudgetManager::new(100);
        let allocation = manager.allocate();

        assert_eq!(allocation.primary, 40);
        assert_eq!(allocation.tests, 20);
        assert_eq!(allocation.callers, 15);
        assert_eq!(allocation.callees, 15);
        assert_eq!(allocation.config, 10);
    }

    #[test]
    fn test_usage_stats() {
        let mut manager = TokenBudgetManager::new(1000);

        manager.reserve("primary", 400);
        manager.reserve("tests", 200);

        let stats = manager.usage_stats();
        assert_eq!(stats.budget, 1000);
        assert_eq!(stats.used, 600);
        assert_eq!(stats.remaining, 400);
        assert_eq!(stats.by_category.len(), 2);
        assert_eq!(stats.by_category.get("primary"), Some(&400));
        assert_eq!(stats.by_category.get("tests"), Some(&200));
    }

    #[test]
    fn test_usage_stats_empty() {
        let manager = TokenBudgetManager::new(1000);
        let stats = manager.usage_stats();

        assert_eq!(stats.budget, 1000);
        assert_eq!(stats.used, 0);
        assert_eq!(stats.remaining, 1000);
        assert!(stats.by_category.is_empty());
    }

    #[test]
    fn test_multiple_categories() {
        let mut manager = TokenBudgetManager::new(2000);

        manager.reserve("primary", 600);
        manager.reserve("tests", 300);
        manager.reserve("callers", 200);
        manager.reserve("callees", 200);
        manager.reserve("config", 100);

        assert_eq!(manager.used(), 1400);
        assert_eq!(manager.remaining(), 600);

        let stats = manager.usage_stats();
        assert_eq!(stats.by_category.len(), 5);
    }

    #[test]
    fn test_budget_allocation_total() {
        let allocation = BudgetAllocation {
            primary: 1000,
            tests: 500,
            callers: 300,
            callees: 300,
            config: 200,
        };

        assert_eq!(allocation.total(), 2300);
    }

    #[test]
    fn test_remaining_with_underflow_protection() {
        let mut manager = TokenBudgetManager::new(100);

        manager.reserve("primary", 100);
        assert_eq!(manager.remaining(), 0);

        // Saturating sub prevents underflow
        let remaining = manager.remaining();
        assert_eq!(remaining, 0);
    }
}
