# Flask extensions.py example - Common Flask extension patterns
from flask_sqlalchemy import SQLAlchemy
from flask_login import LoginManager
from flask_mail import Mail, Message
from flask_caching import Cache
from flask_wtf.csrf import CSRFProtect
from flask_migrate import Migrate
from flask_limiter import Limiter
from flask_limiter.util import get_remote_address
from flask_cors import CORS
from werkzeug.local import LocalProxy
import logging
from functools import wraps


# Initialize extensions (without app binding)
db = SQLAlchemy()
login_manager = LoginManager()
mail = Mail()
cache = Cache()
csrf = CSRFProtect()
migrate = Migrate()
limiter = Limiter(
    key_func=get_remote_address,
    default_limits=["200 per day", "50 per hour"]
)
cors = CORS()


def init_extensions(app):
    """Initialize all extensions with the Flask app.

    Args:
        app: Flask application instance

    Returns:
        None
    """
    # Database
    db.init_app(app)

    # Login manager
    login_manager.init_app(app)
    login_manager.login_view = 'auth.login'
    login_manager.login_message = 'Please log in to access this page.'
    login_manager.login_message_category = 'info'

    # Mail
    mail.init_app(app)

    # Cache
    cache.init_app(app, config={
        'CACHE_TYPE': 'simple',
        'CACHE_DEFAULT_TIMEOUT': 300
    })

    # CSRF Protection
    csrf.init_app(app)

    # Database migrations
    migrate.init_app(app, db)

    # Rate limiting
    limiter.init_app(app)

    # CORS
    cors.init_app(app, resources={
        r"/api/*": {
            "origins": "*",
            "methods": ["GET", "POST", "PUT", "DELETE"],
            "allow_headers": ["Content-Type", "Authorization"]
        }
    })

    # Logging
    configure_logging(app)


def configure_logging(app):
    """Configure application logging.

    Args:
        app: Flask application instance
    """
    if not app.debug and not app.testing:
        # File handler
        file_handler = logging.FileHandler('app.log')
        file_handler.setLevel(logging.INFO)
        file_handler.setFormatter(logging.Formatter(
            '[%(asctime)s] %(levelname)s in %(module)s: %(message)s'
        ))
        app.logger.addHandler(file_handler)

        # Console handler
        console_handler = logging.StreamHandler()
        console_handler.setLevel(logging.INFO)
        app.logger.addHandler(console_handler)

        app.logger.setLevel(logging.INFO)
        app.logger.info('Application startup')


# User loader for Flask-Login
@login_manager.user_loader
def load_user(user_id):
    """Load user by ID for Flask-Login.

    Args:
        user_id (int): User ID

    Returns:
        User: User object or None
    """
    from .app import User
    return User.query.get(int(user_id))


# Custom cache decorator
def cached(timeout=300, key_prefix='view'):
    """Custom cache decorator with flexible key generation.

    Args:
        timeout (int): Cache timeout in seconds
        key_prefix (str): Prefix for cache key

    Returns:
        function: Decorated function
    """
    def decorator(f):
        @wraps(f)
        def decorated_function(*args, **kwargs):
            cache_key = f'{key_prefix}::{f.__name__}'
            rv = cache.get(cache_key)
            if rv is not None:
                return rv
            rv = f(*args, **kwargs)
            cache.set(cache_key, rv, timeout=timeout)
            return rv
        return decorated_function
    return decorator


# Email helper functions
def send_email(subject, recipients, text_body, html_body=None):
    """Send email using Flask-Mail.

    Args:
        subject (str): Email subject
        recipients (list): List of recipient email addresses
        text_body (str): Plain text email body
        html_body (str, optional): HTML email body

    Returns:
        None
    """
    msg = Message(subject, recipients=recipients)
    msg.body = text_body
    if html_body:
        msg.html = html_body
    mail.send(msg)


def send_async_email(app, subject, recipients, text_body, html_body=None):
    """Send email asynchronously.

    Args:
        app: Flask application instance
        subject (str): Email subject
        recipients (list): List of recipient email addresses
        text_body (str): Plain text email body
        html_body (str, optional): HTML email body

    Returns:
        None
    """
    import threading

    def send_message(app, msg):
        with app.app_context():
            mail.send(msg)

    msg = Message(subject, recipients=recipients)
    msg.body = text_body
    if html_body:
        msg.html = html_body

    thread = threading.Thread(target=send_message, args=(app, msg))
    thread.start()


def send_password_reset_email(user):
    """Send password reset email to user.

    Args:
        user: User object

    Returns:
        None
    """
    from flask import current_app, url_for

    token = user.generate_reset_token()
    send_email(
        'Password Reset Request',
        recipients=[user.email],
        text_body=f'''To reset your password, visit the following link:
{url_for('auth.reset_password', token=token, _external=True)}

If you did not request a password reset, simply ignore this email.
        ''',
        html_body=f'''
<p>To reset your password, click the link below:</p>
<p><a href="{url_for('auth.reset_password', token=token, _external=True)}">Reset Password</a></p>
<p>If you did not request a password reset, simply ignore this email.</p>
        '''
    )


# Cache management
class CacheManager:
    """Manager for cache operations."""

    @staticmethod
    def clear_all():
        """Clear all cached data."""
        cache.clear()

    @staticmethod
    def clear_user_cache(user_id):
        """Clear cache for specific user.

        Args:
            user_id (int): User ID
        """
        pattern = f'user:{user_id}:*'
        # Note: This is a simplified example
        cache.delete_many(pattern)

    @staticmethod
    def warm_cache():
        """Pre-populate cache with frequently accessed data."""
        from .app import Post

        # Cache recent posts
        posts = Post.query.filter_by(published=True).limit(10).all()
        cache.set('recent_posts', posts, timeout=600)

    @staticmethod
    def get_or_set(key, func, timeout=300):
        """Get from cache or set if not exists.

        Args:
            key (str): Cache key
            func (callable): Function to call if cache miss
            timeout (int): Cache timeout in seconds

        Returns:
            any: Cached or computed value
        """
        value = cache.get(key)
        if value is None:
            value = func()
            cache.set(key, value, timeout=timeout)
        return value


# Rate limiting helpers
def rate_limit(limit_string):
    """Custom rate limit decorator.

    Args:
        limit_string (str): Rate limit string (e.g., "10 per minute")

    Returns:
        function: Decorated function
    """
    def decorator(f):
        return limiter.limit(limit_string)(f)
    return decorator


# Database helpers
class DatabaseManager:
    """Manager for database operations."""

    @staticmethod
    def create_all():
        """Create all database tables."""
        db.create_all()

    @staticmethod
    def drop_all():
        """Drop all database tables."""
        db.drop_all()

    @staticmethod
    def reset():
        """Reset database (drop and recreate)."""
        db.drop_all()
        db.create_all()

    @staticmethod
    def seed_data():
        """Seed database with initial data."""
        from .app import User, Post, Tag

        # Create admin user
        admin = User(username='admin', email='admin@example.com', is_admin=True)
        admin.set_password('admin123')
        db.session.add(admin)

        # Create tags
        tags = [
            Tag(name='Python'),
            Tag(name='Flask'),
            Tag(name='Web Development'),
        ]
        for tag in tags:
            db.session.add(tag)

        db.session.commit()


# CORS helpers
def configure_cors(app):
    """Configure CORS with custom settings.

    Args:
        app: Flask application instance
    """
    CORS(app, resources={
        r"/api/*": {
            "origins": app.config.get('CORS_ORIGINS', '*'),
            "methods": ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
            "allow_headers": ["Content-Type", "Authorization", "X-API-Key"],
            "expose_headers": ["Content-Range", "X-Total-Count"],
            "max_age": 600
        }
    })


# Error tracking
class ErrorTracker:
    """Track and report application errors."""

    @staticmethod
    def log_error(error, context=None):
        """Log error with context.

        Args:
            error: Exception object
            context (dict, optional): Additional context
        """
        from flask import current_app

        current_app.logger.error(
            f'Error: {str(error)}',
            extra={'context': context or {}}
        )

    @staticmethod
    def send_error_notification(error):
        """Send error notification to administrators.

        Args:
            error: Exception object
        """
        from flask import current_app

        if current_app.config.get('SEND_ERROR_EMAILS'):
            admin_emails = current_app.config.get('ADMIN_EMAILS', [])
            if admin_emails:
                send_email(
                    'Application Error',
                    recipients=admin_emails,
                    text_body=f'An error occurred:\n\n{str(error)}'
                )


# Utility functions
def get_current_app():
    """Get current Flask app instance.

    Returns:
        Flask: Current Flask application
    """
    from flask import current_app
    return current_app


# Export commonly used proxies
current_db = LocalProxy(lambda: db)
current_cache = LocalProxy(lambda: cache)
