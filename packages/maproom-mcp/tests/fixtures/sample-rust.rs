/// Sample Rust file for E2E testing
/// This file contains typical Rust patterns

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

pub struct UserRepository {
    users: HashMap<u32, User>,
    next_id: u32,
}

impl UserRepository {
    /// Create a new UserRepository
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add a new user to the repository
    pub fn create_user(&mut self, name: String, email: String) -> User {
        let user = User {
            id: self.next_id,
            name,
            email,
        };
        self.users.insert(self.next_id, user.clone());
        self.next_id += 1;
        user
    }

    /// Find a user by ID
    pub fn find_by_id(&self, id: u32) -> Option<&User> {
        self.users.get(&id)
    }

    /// Delete a user by ID
    pub fn delete_user(&mut self, id: u32) -> bool {
        self.users.remove(&id).is_some()
    }

    /// Get all users
    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
}

/// Validate email format
pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

/// Format user for display
pub fn format_user(user: &User) -> String {
    format!("{} <{}>", user.name, user.email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user() {
        let mut repo = UserRepository::new();
        let user = repo.create_user("Test User".to_string(), "test@example.com".to_string());
        assert_eq!(user.id, 1);
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com"));
        assert!(!validate_email("invalid-email"));
    }
}
