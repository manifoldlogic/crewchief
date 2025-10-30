"""
Sample API module demonstrating various Python constructs.
"""

from typing import List, Optional, Dict, Any
from dataclasses import dataclass
from abc import ABC, abstractmethod

# Module-level constants
API_VERSION = "1.0.0"
DEFAULT_TIMEOUT = 30
MAX_RETRIES = 3


@dataclass
class Request:
    """HTTP request model."""
    url: str
    method: str = "GET"
    headers: Optional[Dict[str, str]] = None
    body: Optional[str] = None


@dataclass
class Response:
    """HTTP response model."""
    status_code: int
    body: str
    headers: Dict[str, str]


class APIException(Exception):
    """Base exception for API errors."""

    def __init__(self, message: str, status_code: int):
        """Initialize the exception."""
        super().__init__(message)
        self.status_code = status_code


class BaseClient(ABC):
    """Abstract base class for API clients."""

    def __init__(self, base_url: str, timeout: int = DEFAULT_TIMEOUT):
        """Initialize the client."""
        self.base_url = base_url
        self.timeout = timeout

    @abstractmethod
    async def send_request(self, request: Request) -> Response:
        """Send an HTTP request."""
        pass

    @property
    def api_version(self) -> str:
        """Get the API version."""
        return API_VERSION


class HTTPClient(BaseClient):
    """HTTP client implementation."""

    def __init__(self, base_url: str, api_key: Optional[str] = None):
        """Initialize the HTTP client."""
        super().__init__(base_url)
        self.api_key = api_key
        self._session = None

    async def send_request(self, request: Request) -> Response:
        """Send an HTTP request."""
        return await self._execute_request(request)

    async def _execute_request(self, request: Request) -> Response:
        """Execute the HTTP request."""
        # Implementation here
        pass

    @staticmethod
    def validate_url(url: str) -> bool:
        """Validate a URL."""
        return url.startswith(("http://", "https://"))

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "HTTPClient":
        """Create client from configuration."""
        return cls(
            base_url=config["base_url"],
            api_key=config.get("api_key")
        )


async def fetch_data(client: HTTPClient, endpoint: str) -> Dict[str, Any]:
    """Fetch data from an API endpoint."""
    request = Request(url=f"{client.base_url}/{endpoint}")
    response = await client.send_request(request)
    return parse_response(response)


def parse_response(response: Response) -> Dict[str, Any]:
    """Parse API response."""
    if response.status_code >= 400:
        raise APIException(
            f"Request failed with status {response.status_code}",
            response.status_code
        )

    import json
    return json.loads(response.body)


def retry(max_attempts: int = MAX_RETRIES):
    """Decorator for retrying failed requests."""
    def decorator(func):
        async def wrapper(*args, **kwargs):
            for attempt in range(max_attempts):
                try:
                    return await func(*args, **kwargs)
                except APIException:
                    if attempt == max_attempts - 1:
                        raise
            return None
        return wrapper
    return decorator
