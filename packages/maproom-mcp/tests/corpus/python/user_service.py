"""
User service module for CRUD operations on user accounts.
Provides create, read, update, and delete operations for users.
"""

from dataclasses import dataclass
from typing import Optional, List


@dataclass
class User:
    """User data model representing an account in the system."""
    id: str
    username: str
    email: str
    created_at: int


class UserService:
    """
    UserService handles all user CRUD operations.
    Provides methods to create, read, update, and delete user accounts.
    """

    def __init__(self, database):
        self.database = database
        self._users: dict = {}

    def create_user(self, username: str, email: str) -> User:
        """
        Create a new user account.

        Args:
            username: The username for the new account
            email: The email address for the new account

        Returns:
            The created User object
        """
        import time
        user_id = f"user_{len(self._users) + 1}"
        user = User(
            id=user_id,
            username=username,
            email=email,
            created_at=int(time.time())
        )
        self._users[user_id] = user
        return user

    def get_user(self, user_id: str) -> Optional[User]:
        """
        Get a user by their ID.

        Args:
            user_id: The unique identifier of the user

        Returns:
            User object if found, None otherwise
        """
        return self._users.get(user_id)

    def get_user_by_email(self, email: str) -> Optional[User]:
        """Find a user by their email address."""
        for user in self._users.values():
            if user.email == email:
                return user
        return None

    def update_user(self, user_id: str, username: str = None, email: str = None) -> Optional[User]:
        """
        Update a user's information.

        Args:
            user_id: The ID of the user to update
            username: New username (optional)
            email: New email (optional)

        Returns:
            Updated User object if found, None otherwise
        """
        user = self.get_user(user_id)
        if user is None:
            return None
        if username:
            user.username = username
        if email:
            user.email = email
        return user

    def delete_user(self, user_id: str) -> bool:
        """
        Delete a user account.

        Args:
            user_id: The ID of the user to delete

        Returns:
            True if deleted, False if user not found
        """
        if user_id in self._users:
            del self._users[user_id]
            return True
        return False

    def list_users(self) -> List[User]:
        """Get all users in the system."""
        return list(self._users.values())
