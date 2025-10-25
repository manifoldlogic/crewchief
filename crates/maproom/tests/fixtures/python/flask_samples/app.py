# Flask app.py example - Real-world Flask application patterns
from flask import Flask, render_template, request, redirect, url_for, flash, jsonify, session, abort
from flask_sqlalchemy import SQLAlchemy
from flask_login import LoginManager, UserMixin, login_user, logout_user, login_required, current_user
from flask_wtf import FlaskForm
from werkzeug.security import generate_password_hash, check_password_hash
from functools import wraps
from datetime import datetime
import os


# Create Flask app
app = Flask(__name__)
app.config['SECRET_KEY'] = os.environ.get('SECRET_KEY', 'dev-secret-key')
app.config['SQLALCHEMY_DATABASE_URI'] = os.environ.get('DATABASE_URL', 'sqlite:///app.db')
app.config['SQLALCHEMY_TRACK_MODIFICATIONS'] = False

# Initialize extensions
db = SQLAlchemy(app)
login_manager = LoginManager(app)
login_manager.login_view = 'login'
login_manager.login_message = 'Please log in to access this page.'


# Models
class User(UserMixin, db.Model):
    """User model."""

    __tablename__ = 'users'

    id = db.Column(db.Integer, primary_key=True)
    username = db.Column(db.String(80), unique=True, nullable=False)
    email = db.Column(db.String(120), unique=True, nullable=False)
    password_hash = db.Column(db.String(200), nullable=False)
    is_admin = db.Column(db.Boolean, default=False)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    posts = db.relationship('Post', backref='author', lazy='dynamic')

    def set_password(self, password):
        """Set hashed password."""
        self.password_hash = generate_password_hash(password)

    def check_password(self, password):
        """Check password against hash."""
        return check_password_hash(self.password_hash, password)

    def __repr__(self):
        return f'<User {self.username}>'


class Post(db.Model):
    """Blog post model."""

    __tablename__ = 'posts'

    id = db.Column(db.Integer, primary_key=True)
    title = db.Column(db.String(200), nullable=False)
    slug = db.Column(db.String(200), unique=True, nullable=False)
    content = db.Column(db.Text, nullable=False)
    published = db.Column(db.Boolean, default=False)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    user_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    tags = db.relationship('Tag', secondary='post_tags', backref='posts')

    def __repr__(self):
        return f'<Post {self.title}>'


class Tag(db.Model):
    """Tag model."""

    __tablename__ = 'tags'

    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(50), unique=True, nullable=False)

    def __repr__(self):
        return f'<Tag {self.name}>'


# Association table for many-to-many relationship
post_tags = db.Table(
    'post_tags',
    db.Column('post_id', db.Integer, db.ForeignKey('posts.id'), primary_key=True),
    db.Column('tag_id', db.Integer, db.ForeignKey('tags.id'), primary_key=True)
)


# Login manager user loader
@login_manager.user_loader
def load_user(user_id):
    """Load user by ID."""
    return User.query.get(int(user_id))


# Custom decorators
def admin_required(f):
    """Decorator to require admin access."""
    @wraps(f)
    def decorated_function(*args, **kwargs):
        if not current_user.is_authenticated or not current_user.is_admin:
            abort(403)
        return f(*args, **kwargs)
    return decorated_function


def api_key_required(f):
    """Decorator to require API key."""
    @wraps(f)
    def decorated_function(*args, **kwargs):
        api_key = request.headers.get('X-API-Key')
        if not api_key or api_key != app.config.get('API_KEY'):
            return jsonify({'error': 'Invalid API key'}), 401
        return f(*args, **kwargs)
    return decorated_function


# Routes
@app.route('/')
def index():
    """Home page."""
    posts = Post.query.filter_by(published=True).order_by(Post.created_at.desc()).limit(10).all()
    return render_template('index.html', posts=posts)


@app.route('/about')
def about():
    """About page."""
    return render_template('about.html')


@app.route('/register', methods=['GET', 'POST'])
def register():
    """User registration."""
    if current_user.is_authenticated:
        return redirect(url_for('index'))

    if request.method == 'POST':
        username = request.form.get('username')
        email = request.form.get('email')
        password = request.form.get('password')

        # Validate input
        if not username or not email or not password:
            flash('All fields are required.', 'error')
            return render_template('register.html')

        # Check if user exists
        if User.query.filter_by(username=username).first():
            flash('Username already exists.', 'error')
            return render_template('register.html')

        if User.query.filter_by(email=email).first():
            flash('Email already registered.', 'error')
            return render_template('register.html')

        # Create user
        user = User(username=username, email=email)
        user.set_password(password)
        db.session.add(user)
        db.session.commit()

        flash('Registration successful! Please log in.', 'success')
        return redirect(url_for('login'))

    return render_template('register.html')


@app.route('/login', methods=['GET', 'POST'])
def login():
    """User login."""
    if current_user.is_authenticated:
        return redirect(url_for('index'))

    if request.method == 'POST':
        username = request.form.get('username')
        password = request.form.get('password')
        remember = request.form.get('remember', False)

        user = User.query.filter_by(username=username).first()

        if user and user.check_password(password):
            login_user(user, remember=remember)
            flash('Logged in successfully!', 'success')
            next_page = request.args.get('next')
            return redirect(next_page or url_for('index'))

        flash('Invalid username or password.', 'error')

    return render_template('login.html')


@app.route('/logout')
@login_required
def logout():
    """User logout."""
    logout_user()
    flash('Logged out successfully.', 'success')
    return redirect(url_for('index'))


@app.route('/posts')
def posts():
    """List all published posts."""
    page = request.args.get('page', 1, type=int)
    tag_name = request.args.get('tag')

    query = Post.query.filter_by(published=True)

    if tag_name:
        tag = Tag.query.filter_by(name=tag_name).first_or_404()
        query = query.filter(Post.tags.contains(tag))

    posts = query.order_by(Post.created_at.desc()).paginate(
        page=page, per_page=10, error_out=False
    )

    return render_template('posts.html', posts=posts, current_tag=tag_name)


@app.route('/posts/<slug>')
def post_detail(slug):
    """View single post."""
    post = Post.query.filter_by(slug=slug, published=True).first_or_404()
    return render_template('post_detail.html', post=post)


@app.route('/posts/new', methods=['GET', 'POST'])
@login_required
def create_post():
    """Create a new post."""
    if request.method == 'POST':
        title = request.form.get('title')
        content = request.form.get('content')
        slug = request.form.get('slug')
        published = request.form.get('published') == 'on'

        if not title or not content or not slug:
            flash('Title, content, and slug are required.', 'error')
            return render_template('post_form.html')

        # Check if slug is unique
        if Post.query.filter_by(slug=slug).first():
            flash('Slug already exists.', 'error')
            return render_template('post_form.html')

        post = Post(
            title=title,
            content=content,
            slug=slug,
            published=published,
            author=current_user
        )
        db.session.add(post)
        db.session.commit()

        flash('Post created successfully!', 'success')
        return redirect(url_for('post_detail', slug=post.slug))

    return render_template('post_form.html')


@app.route('/posts/<slug>/edit', methods=['GET', 'POST'])
@login_required
def edit_post(slug):
    """Edit an existing post."""
    post = Post.query.filter_by(slug=slug).first_or_404()

    # Check permissions
    if post.author != current_user and not current_user.is_admin:
        abort(403)

    if request.method == 'POST':
        post.title = request.form.get('title')
        post.content = request.form.get('content')
        post.published = request.form.get('published') == 'on'
        post.updated_at = datetime.utcnow()

        db.session.commit()
        flash('Post updated successfully!', 'success')
        return redirect(url_for('post_detail', slug=post.slug))

    return render_template('post_form.html', post=post)


@app.route('/posts/<slug>/delete', methods=['POST'])
@login_required
def delete_post(slug):
    """Delete a post."""
    post = Post.query.filter_by(slug=slug).first_or_404()

    # Check permissions
    if post.author != current_user and not current_user.is_admin:
        abort(403)

    db.session.delete(post)
    db.session.commit()
    flash('Post deleted successfully!', 'success')
    return redirect(url_for('posts'))


@app.route('/admin')
@admin_required
def admin_dashboard():
    """Admin dashboard."""
    user_count = User.query.count()
    post_count = Post.query.count()
    published_count = Post.query.filter_by(published=True).count()

    return render_template(
        'admin/dashboard.html',
        user_count=user_count,
        post_count=post_count,
        published_count=published_count
    )


# API endpoints
@app.route('/api/posts')
def api_posts():
    """API endpoint to get posts as JSON."""
    page = request.args.get('page', 1, type=int)
    per_page = request.args.get('per_page', 10, type=int)

    posts = Post.query.filter_by(published=True).paginate(
        page=page, per_page=per_page, error_out=False
    )

    return jsonify({
        'posts': [
            {
                'id': post.id,
                'title': post.title,
                'slug': post.slug,
                'content': post.content[:200],
                'author': post.author.username,
                'created_at': post.created_at.isoformat(),
            }
            for post in posts.items
        ],
        'page': page,
        'total': posts.total,
        'pages': posts.pages,
    })


@app.route('/api/posts/<slug>')
def api_post_detail(slug):
    """API endpoint to get single post as JSON."""
    post = Post.query.filter_by(slug=slug, published=True).first_or_404()

    return jsonify({
        'id': post.id,
        'title': post.title,
        'slug': post.slug,
        'content': post.content,
        'author': post.author.username,
        'created_at': post.created_at.isoformat(),
        'updated_at': post.updated_at.isoformat(),
        'tags': [tag.name for tag in post.tags],
    })


@app.route('/api/stats')
@api_key_required
def api_stats():
    """Protected API endpoint for statistics."""
    return jsonify({
        'users': User.query.count(),
        'posts': Post.query.count(),
        'published_posts': Post.query.filter_by(published=True).count(),
        'tags': Tag.query.count(),
    })


# Error handlers
@app.errorhandler(404)
def not_found(error):
    """404 error handler."""
    return render_template('errors/404.html'), 404


@app.errorhandler(403)
def forbidden(error):
    """403 error handler."""
    return render_template('errors/403.html'), 403


@app.errorhandler(500)
def internal_error(error):
    """500 error handler."""
    db.session.rollback()
    return render_template('errors/500.html'), 500


# Template filters
@app.template_filter('datetimeformat')
def datetimeformat(value, format='%Y-%m-%d %H:%M'):
    """Format a datetime object."""
    if value is None:
        return ''
    return value.strftime(format)


@app.template_filter('truncate_words')
def truncate_words(value, num_words=50):
    """Truncate text to specified number of words."""
    words = value.split()
    if len(words) > num_words:
        return ' '.join(words[:num_words]) + '...'
    return value


# CLI commands
@app.cli.command()
def initdb():
    """Initialize the database."""
    db.create_all()
    print('Database initialized.')


@app.cli.command()
def createadmin():
    """Create an admin user."""
    admin = User(username='admin', email='admin@example.com', is_admin=True)
    admin.set_password('admin123')
    db.session.add(admin)
    db.session.commit()
    print('Admin user created.')


if __name__ == '__main__':
    app.run(debug=True)
