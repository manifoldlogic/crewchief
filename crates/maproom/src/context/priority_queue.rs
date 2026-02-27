//! Priority queue for selecting chunks by relevance and importance.
//!
//! This module provides a priority queue data structure for ordering code chunks
//! during context assembly. Chunks are ordered by a priority score that combines:
//! - Search relevance scores
//! - Category importance weights
//! - Distance from the primary chunk
//!
//! The queue enables the assembler to select the most valuable chunks first
//! when working within token budget constraints.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Category of a context chunk, used for priority weighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Primary chunk being explained
    Primary,
    /// Test chunk demonstrating usage
    Test,
    /// Caller chunk (function that calls the primary)
    Caller,
    /// Callee chunk (function called by the primary)
    Callee,
    /// Configuration or metadata
    Config,
    /// Documentation
    Doc,
}

impl Category {
    /// Get the importance weight for this category.
    ///
    /// Higher weights mean higher priority in the queue.
    /// Weights are calibrated to match budget allocation percentages:
    /// - Primary: 1.0 (highest priority)
    /// - Test: 0.8 (high priority for understanding usage)
    /// - Caller: 0.6 (medium-high priority)
    /// - Callee: 0.6 (medium-high priority)
    /// - Doc: 0.4 (medium priority)
    /// - Config: 0.3 (lower priority)
    pub fn weight(&self) -> f64 {
        match self {
            Category::Primary => 1.0,
            Category::Test => 0.8,
            Category::Caller => 0.6,
            Category::Callee => 0.6,
            Category::Doc => 0.4,
            Category::Config => 0.3,
        }
    }
}

/// A prioritized item in the queue.
///
/// Items are ordered by priority score (higher = more important).
/// When priorities are equal, items maintain insertion order (FIFO).
#[derive(Debug, Clone)]
pub struct PriorityItem<T> {
    /// Priority score (higher = more important)
    priority: f64,
    /// Category of this item
    category: Category,
    /// Insertion order for stable sorting
    sequence: usize,
    /// The actual item
    item: T,
}

impl<T> PriorityItem<T> {
    /// Create a new priority item.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::{PriorityItem, Category};
    ///
    /// let item = PriorityItem::new(0.9, Category::Primary, 0, "chunk_data");
    /// assert_eq!(item.priority(), 0.9);
    /// ```
    pub fn new(priority: f64, category: Category, sequence: usize, item: T) -> Self {
        Self {
            priority,
            category,
            sequence,
            item,
        }
    }

    /// Get the priority score.
    pub fn priority(&self) -> f64 {
        self.priority
    }

    /// Get the category.
    pub fn category(&self) -> Category {
        self.category
    }

    /// Get the sequence number.
    pub fn sequence(&self) -> usize {
        self.sequence
    }

    /// Get a reference to the item.
    pub fn item(&self) -> &T {
        &self.item
    }

    /// Consume this PriorityItem and return the inner item.
    pub fn into_inner(self) -> T {
        self.item
    }
}

// Implement Ord for max-heap behavior (highest priority first)
impl<T> Ord for PriorityItem<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by priority (reverse for max-heap)
        match self.priority.partial_cmp(&other.priority) {
            Some(Ordering::Equal) | None => {
                // If priorities are equal, use sequence for stable ordering (FIFO)
                // Reverse sequence so earlier items come first
                other.sequence.cmp(&self.sequence)
            }
            Some(ord) => ord,
        }
    }
}

impl<T> PartialOrd for PriorityItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq for PriorityItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence == other.sequence
    }
}

impl<T> Eq for PriorityItem<T> {}

/// Priority queue for chunk selection during context assembly.
///
/// The queue orders chunks by priority score, which combines:
/// - Base relevance (from search or graph traversal)
/// - Category weight (primary > test > caller/callee > doc > config)
/// - Custom adjustments (e.g., same directory bonus)
///
/// # Example
///
/// ```
/// use maproom::context::priority_queue::{PriorityQueue, Category};
///
/// let mut queue = PriorityQueue::new();
///
/// // Add items with different priorities
/// queue.push(0.9, Category::Primary, "main_function");
/// queue.push(0.7, Category::Test, "test_main");
/// queue.push(0.6, Category::Caller, "caller_fn");
///
/// // Items come out in priority order
/// assert_eq!(queue.pop().unwrap().category(), Category::Primary);
/// assert_eq!(queue.pop().unwrap().category(), Category::Test);
/// assert_eq!(queue.pop().unwrap().category(), Category::Caller);
/// ```
#[derive(Debug, Clone)]
pub struct PriorityQueue<T> {
    /// Internal binary heap for priority ordering
    heap: BinaryHeap<PriorityItem<T>>,
    /// Sequence counter for stable ordering
    sequence: usize,
}

impl<T> PriorityQueue<T> {
    /// Create a new empty priority queue.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::PriorityQueue;
    ///
    /// let queue: PriorityQueue<String> = PriorityQueue::new();
    /// assert_eq!(queue.len(), 0);
    /// assert!(queue.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            sequence: 0,
        }
    }

    /// Create a new priority queue with the specified capacity.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::PriorityQueue;
    ///
    /// let queue: PriorityQueue<String> = PriorityQueue::with_capacity(100);
    /// assert_eq!(queue.len(), 0);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            heap: BinaryHeap::with_capacity(capacity),
            sequence: 0,
        }
    }

    /// Add an item to the queue with the given priority and category.
    ///
    /// The final priority is calculated as: `base_priority * category_weight`
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::{PriorityQueue, Category};
    ///
    /// let mut queue = PriorityQueue::new();
    /// queue.push(0.9, Category::Primary, "chunk1");
    /// queue.push(0.8, Category::Test, "chunk2");
    ///
    /// assert_eq!(queue.len(), 2);
    /// ```
    pub fn push(&mut self, base_priority: f64, category: Category, item: T) {
        // Calculate weighted priority
        let priority = base_priority * category.weight();

        let priority_item = PriorityItem::new(priority, category, self.sequence, item);
        self.heap.push(priority_item);
        self.sequence += 1;
    }

    /// Remove and return the highest priority item.
    ///
    /// Returns `None` if the queue is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::{PriorityQueue, Category};
    ///
    /// let mut queue = PriorityQueue::new();
    /// queue.push(0.5, Category::Caller, "low");
    /// queue.push(0.9, Category::Primary, "high");
    ///
    /// let item = queue.pop().unwrap();
    /// assert_eq!(item.category(), Category::Primary);
    /// assert_eq!(*item.item(), "high");
    /// ```
    pub fn pop(&mut self) -> Option<PriorityItem<T>> {
        self.heap.pop()
    }

    /// Peek at the highest priority item without removing it.
    ///
    /// Returns `None` if the queue is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::{PriorityQueue, Category};
    ///
    /// let mut queue = PriorityQueue::new();
    /// queue.push(0.9, Category::Primary, "data");
    ///
    /// let item = queue.peek().unwrap();
    /// assert_eq!(*item.item(), "data");
    /// assert_eq!(queue.len(), 1); // Still in queue
    /// ```
    pub fn peek(&self) -> Option<&PriorityItem<T>> {
        self.heap.peek()
    }

    /// Get the number of items in the queue.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::{PriorityQueue, Category};
    ///
    /// let mut queue = PriorityQueue::new();
    /// assert_eq!(queue.len(), 0);
    ///
    /// queue.push(0.9, Category::Primary, "data");
    /// assert_eq!(queue.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Check if the queue is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::PriorityQueue;
    ///
    /// let queue: PriorityQueue<String> = PriorityQueue::new();
    /// assert!(queue.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Remove all items from the queue.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::{PriorityQueue, Category};
    ///
    /// let mut queue = PriorityQueue::new();
    /// queue.push(0.9, Category::Primary, "data");
    /// queue.clear();
    ///
    /// assert!(queue.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.heap.clear();
        self.sequence = 0;
    }

    /// Drain all items in priority order.
    ///
    /// Returns an iterator that removes and yields items from highest to
    /// lowest priority.
    ///
    /// # Example
    ///
    /// ```
    /// use maproom::context::priority_queue::{PriorityQueue, Category};
    ///
    /// let mut queue = PriorityQueue::new();
    /// queue.push(0.5, Category::Caller, "low");
    /// queue.push(0.9, Category::Primary, "high");
    /// queue.push(0.7, Category::Test, "med");
    ///
    /// let items: Vec<_> = queue.drain().map(|i| i.into_inner()).collect();
    /// assert_eq!(items, vec!["high", "med", "low"]);
    /// assert!(queue.is_empty());
    /// ```
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain { queue: self }
    }
}

impl<T> Default for PriorityQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator that drains items from a priority queue in priority order.
pub struct Drain<'a, T> {
    queue: &'a mut PriorityQueue<T>,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = PriorityItem<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop()
    }
}

impl<'a, T> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        // Ensure all remaining items are consumed
        while self.next().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_weights() {
        assert_eq!(Category::Primary.weight(), 1.0);
        assert_eq!(Category::Test.weight(), 0.8);
        assert_eq!(Category::Caller.weight(), 0.6);
        assert_eq!(Category::Callee.weight(), 0.6);
        assert_eq!(Category::Doc.weight(), 0.4);
        assert_eq!(Category::Config.weight(), 0.3);
    }

    #[test]
    fn test_category_weight_ordering() {
        assert!(Category::Primary.weight() > Category::Test.weight());
        assert!(Category::Test.weight() > Category::Caller.weight());
        assert!(Category::Caller.weight() > Category::Doc.weight());
        assert!(Category::Doc.weight() > Category::Config.weight());
    }

    #[test]
    fn test_priority_item_creation() {
        let item = PriorityItem::new(0.9, Category::Primary, 0, "data");
        assert_eq!(item.priority(), 0.9);
        assert_eq!(item.category(), Category::Primary);
        assert_eq!(item.sequence(), 0);
        assert_eq!(*item.item(), "data");
    }

    #[test]
    fn test_priority_item_ordering() {
        let high = PriorityItem::new(0.9, Category::Primary, 0, "high");
        let low = PriorityItem::new(0.5, Category::Caller, 1, "low");

        assert!(high > low);
        assert!(low < high);
    }

    #[test]
    fn test_priority_item_sequence_ordering() {
        // When priorities are equal, earlier sequence comes first
        let first = PriorityItem::new(0.9, Category::Primary, 0, "first");
        let second = PriorityItem::new(0.9, Category::Primary, 1, "second");

        assert!(first > second); // First should have higher priority
    }

    #[test]
    fn test_new_queue() {
        let queue: PriorityQueue<String> = PriorityQueue::new();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_with_capacity() {
        let queue: PriorityQueue<String> = PriorityQueue::with_capacity(100);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_push_and_pop() {
        let mut queue = PriorityQueue::new();

        queue.push(0.9, Category::Primary, "high");
        queue.push(0.5, Category::Caller, "low");
        queue.push(0.7, Category::Test, "med");

        assert_eq!(queue.len(), 3);

        // Items should come out in priority order
        let first = queue.pop().unwrap();
        assert_eq!(*first.item(), "high");
        assert_eq!(first.category(), Category::Primary);

        let second = queue.pop().unwrap();
        assert_eq!(*second.item(), "med");

        let third = queue.pop().unwrap();
        assert_eq!(*third.item(), "low");

        assert!(queue.is_empty());
        assert!(queue.pop().is_none());
    }

    #[test]
    fn test_weighted_priority() {
        let mut queue = PriorityQueue::new();

        // Even though base priorities are equal, category weights differ
        queue.push(1.0, Category::Config, "config"); // 1.0 * 0.3 = 0.3
        queue.push(1.0, Category::Test, "test"); // 1.0 * 0.8 = 0.8
        queue.push(1.0, Category::Primary, "primary"); // 1.0 * 1.0 = 1.0

        assert_eq!(*queue.pop().unwrap().item(), "primary");
        assert_eq!(*queue.pop().unwrap().item(), "test");
        assert_eq!(*queue.pop().unwrap().item(), "config");
    }

    #[test]
    fn test_peek() {
        let mut queue = PriorityQueue::new();

        queue.push(0.5, Category::Caller, "low");
        queue.push(0.9, Category::Primary, "high");

        let item = queue.peek().unwrap();
        assert_eq!(*item.item(), "high");
        assert_eq!(queue.len(), 2); // Still in queue

        queue.pop();
        let item = queue.peek().unwrap();
        assert_eq!(*item.item(), "low");
    }

    #[test]
    fn test_peek_empty() {
        let queue: PriorityQueue<String> = PriorityQueue::new();
        assert!(queue.peek().is_none());
    }

    #[test]
    fn test_clear() {
        let mut queue = PriorityQueue::new();

        queue.push(0.9, Category::Primary, "data1");
        queue.push(0.8, Category::Test, "data2");
        assert_eq!(queue.len(), 2);

        queue.clear();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_drain() {
        let mut queue = PriorityQueue::new();

        queue.push(0.5, Category::Caller, "low");
        queue.push(0.9, Category::Primary, "high");
        queue.push(0.7, Category::Test, "med");

        let items: Vec<_> = queue.drain().map(|i| i.into_inner()).collect();
        assert_eq!(items, vec!["high", "med", "low"]);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_stable_ordering_same_priority() {
        let mut queue = PriorityQueue::new();

        // Add items with same priority and category
        queue.push(1.0, Category::Caller, "first");
        queue.push(1.0, Category::Caller, "second");
        queue.push(1.0, Category::Caller, "third");

        // Should come out in insertion order (FIFO)
        assert_eq!(*queue.pop().unwrap().item(), "first");
        assert_eq!(*queue.pop().unwrap().item(), "second");
        assert_eq!(*queue.pop().unwrap().item(), "third");
    }

    #[test]
    fn test_into_inner() {
        let item = PriorityItem::new(0.9, Category::Primary, 0, String::from("owned"));
        let inner = item.into_inner();
        assert_eq!(inner, "owned");
    }

    #[test]
    fn test_many_items() {
        let mut queue = PriorityQueue::new();

        // Add many items
        for i in 0..1000 {
            let priority = (i as f64) / 1000.0;
            queue.push(priority, Category::Caller, i);
        }

        assert_eq!(queue.len(), 1000);

        // Pop should give highest priority first
        let mut prev_priority = f64::INFINITY;
        while let Some(item) = queue.pop() {
            assert!(item.priority() <= prev_priority);
            prev_priority = item.priority();
        }

        assert!(queue.is_empty());
    }

    #[test]
    fn test_mixed_categories() {
        let mut queue = PriorityQueue::new();

        queue.push(0.8, Category::Config, "config"); // 0.8 * 0.3 = 0.24
        queue.push(0.6, Category::Primary, "primary"); // 0.6 * 1.0 = 0.6
        queue.push(0.7, Category::Test, "test"); // 0.7 * 0.8 = 0.56
        queue.push(0.9, Category::Caller, "caller"); // 0.9 * 0.6 = 0.54

        assert_eq!(*queue.pop().unwrap().item(), "primary"); // 0.6
        assert_eq!(*queue.pop().unwrap().item(), "test"); // 0.56
        assert_eq!(*queue.pop().unwrap().item(), "caller"); // 0.54
        assert_eq!(*queue.pop().unwrap().item(), "config"); // 0.24
    }
}
