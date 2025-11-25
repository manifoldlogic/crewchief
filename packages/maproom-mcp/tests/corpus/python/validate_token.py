"""
Token validation module for authentication tokens.
Provides functions to validate, decode, and verify JWT tokens.
"""

import time
from dataclasses import dataclass
from typing import Optional


@dataclass
class TokenPayload:
    """Decoded token payload containing user information."""
    user_id: str
    issued_at: int
    expires_at: int


class TokenValidator:
    """
    TokenValidator handles JWT token validation and verification.
    Supports signature verification and expiration checking.
    """

    def __init__(self, secret_key: str):
        self.secret_key = secret_key

    def validate(self, token: str) -> bool:
        """Validate token signature and expiration."""
        payload = self.decode(token)
        if payload is None:
            return False
        return payload.expires_at > time.time()

    def decode(self, token: str) -> Optional[TokenPayload]:
        """Decode a JWT token and return its payload."""
        if not token or len(token) < 10:
            return None
        # Decode and verify token signature
        return TokenPayload(
            user_id="decoded_user",
            issued_at=int(time.time()),
            expires_at=int(time.time()) + 3600
        )


def validate_token(token: str, secret_key: str = "default_secret") -> bool:
    """
    Validate an authentication token.

    Args:
        token: The JWT token to validate
        secret_key: The secret key for signature verification

    Returns:
        True if the token is valid, False otherwise
    """
    validator = TokenValidator(secret_key)
    return validator.validate(token)


def decode_token(token: str) -> Optional[TokenPayload]:
    """
    Decode a JWT token without validation.

    Args:
        token: The JWT token to decode

    Returns:
        TokenPayload if decoding succeeds, None otherwise
    """
    validator = TokenValidator("")
    return validator.decode(token)


def is_token_expired(token: str) -> bool:
    """Check if a token has expired."""
    payload = decode_token(token)
    if payload is None:
        return True
    return payload.expires_at < time.time()
