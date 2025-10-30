# Flask blueprints.py example - Blueprint patterns for modular Flask apps
from flask import Blueprint, render_template, request, redirect, url_for, flash, jsonify, current_app
from flask_login import login_required, current_user
from functools import wraps
from datetime import datetime


# API Blueprint
api_bp = Blueprint('api', __name__, url_prefix='/api/v1')


@api_bp.route('/status')
def status():
    """API status endpoint."""
    return jsonify({
        'status': 'ok',
        'version': '1.0',
        'timestamp': datetime.utcnow().isoformat(),
    })


@api_bp.route('/health')
def health():
    """Health check endpoint."""
    return jsonify({
        'status': 'healthy',
        'database': 'connected',
        'uptime': 'unknown',
    })


@api_bp.route('/users')
@login_required
def list_users():
    """List users API."""
    from .app import User

    users = User.query.limit(100).all()
    return jsonify({
        'users': [
            {
                'id': user.id,
                'username': user.username,
                'email': user.email,
                'created_at': user.created_at.isoformat(),
            }
            for user in users
        ]
    })


@api_bp.route('/users/<int:user_id>')
def get_user(user_id):
    """Get user by ID."""
    from .app import User

    user = User.query.get_or_404(user_id)
    return jsonify({
        'id': user.id,
        'username': user.username,
        'email': user.email,
        'is_admin': user.is_admin,
        'created_at': user.created_at.isoformat(),
        'post_count': user.posts.count(),
    })


# Blog Blueprint
blog_bp = Blueprint('blog', __name__, url_prefix='/blog')


@blog_bp.route('/')
def index():
    """Blog index page."""
    from .app import Post

    posts = Post.query.filter_by(published=True).order_by(Post.created_at.desc()).all()
    return render_template('blog/index.html', posts=posts)


@blog_bp.route('/post/<slug>')
def post_detail(slug):
    """Blog post detail."""
    from .app import Post

    post = Post.query.filter_by(slug=slug, published=True).first_or_404()
    return render_template('blog/post.html', post=post)


@blog_bp.route('/create', methods=['GET', 'POST'])
@login_required
def create_post():
    """Create blog post."""
    from .app import Post, db

    if request.method == 'POST':
        title = request.form.get('title')
        content = request.form.get('content')
        slug = request.form.get('slug')

        post = Post(
            title=title,
            content=content,
            slug=slug,
            author=current_user,
            published=True
        )
        db.session.add(post)
        db.session.commit()

        flash('Post created!', 'success')
        return redirect(url_for('blog.post_detail', slug=post.slug))

    return render_template('blog/create.html')


# Admin Blueprint
admin_bp = Blueprint('admin', __name__, url_prefix='/admin')


def admin_required(f):
    """Decorator for admin-only routes."""
    @wraps(f)
    def decorated_function(*args, **kwargs):
        if not current_user.is_authenticated or not current_user.is_admin:
            flash('Admin access required.', 'error')
            return redirect(url_for('index'))
        return f(*args, **kwargs)
    return decorated_function


@admin_bp.before_request
@admin_required
def before_request():
    """Check admin access for all admin routes."""
    pass


@admin_bp.route('/')
def dashboard():
    """Admin dashboard."""
    from .app import User, Post, Tag

    stats = {
        'total_users': User.query.count(),
        'total_posts': Post.query.count(),
        'published_posts': Post.query.filter_by(published=True).count(),
        'total_tags': Tag.query.count(),
    }
    return render_template('admin/dashboard.html', stats=stats)


@admin_bp.route('/users')
def users():
    """User management."""
    from .app import User

    users = User.query.all()
    return render_template('admin/users.html', users=users)


@admin_bp.route('/posts')
def posts():
    """Post management."""
    from .app import Post

    posts = Post.query.order_by(Post.created_at.desc()).all()
    return render_template('admin/posts.html', posts=posts)


@admin_bp.route('/posts/<int:post_id>/publish', methods=['POST'])
def publish_post(post_id):
    """Publish or unpublish a post."""
    from .app import Post, db

    post = Post.query.get_or_404(post_id)
    post.published = not post.published
    db.session.commit()

    flash(f'Post {"published" if post.published else "unpublished"}.', 'success')
    return redirect(url_for('admin.posts'))


# Auth Blueprint
auth_bp = Blueprint('auth', __name__, url_prefix='/auth')


@auth_bp.route('/register', methods=['GET', 'POST'])
def register():
    """User registration."""
    if current_user.is_authenticated:
        return redirect(url_for('index'))

    if request.method == 'POST':
        from .app import User, db

        username = request.form.get('username')
        email = request.form.get('email')
        password = request.form.get('password')

        # Validation
        if User.query.filter_by(username=username).first():
            flash('Username already exists.', 'error')
            return render_template('auth/register.html')

        user = User(username=username, email=email)
        user.set_password(password)
        db.session.add(user)
        db.session.commit()

        flash('Registration successful!', 'success')
        return redirect(url_for('auth.login'))

    return render_template('auth/register.html')


@auth_bp.route('/login', methods=['GET', 'POST'])
def login():
    """User login."""
    if current_user.is_authenticated:
        return redirect(url_for('index'))

    if request.method == 'POST':
        from flask_login import login_user
        from .app import User

        username = request.form.get('username')
        password = request.form.get('password')

        user = User.query.filter_by(username=username).first()

        if user and user.check_password(password):
            login_user(user)
            flash('Logged in successfully!', 'success')
            return redirect(url_for('index'))

        flash('Invalid credentials.', 'error')

    return render_template('auth/login.html')


@auth_bp.route('/logout')
@login_required
def logout():
    """User logout."""
    from flask_login import logout_user

    logout_user()
    flash('Logged out.', 'success')
    return redirect(url_for('index'))


@auth_bp.route('/profile')
@login_required
def profile():
    """User profile."""
    return render_template('auth/profile.html', user=current_user)


@auth_bp.route('/profile/edit', methods=['GET', 'POST'])
@login_required
def edit_profile():
    """Edit user profile."""
    from .app import db

    if request.method == 'POST':
        current_user.email = request.form.get('email')
        db.session.commit()
        flash('Profile updated!', 'success')
        return redirect(url_for('auth.profile'))

    return render_template('auth/edit_profile.html')


# Search Blueprint
search_bp = Blueprint('search', __name__, url_prefix='/search')


@search_bp.route('/')
def search():
    """Search functionality."""
    from .app import Post

    query = request.args.get('q', '')
    if not query:
        return render_template('search/results.html', posts=[], query='')

    posts = Post.query.filter(
        Post.published == True,
        (Post.title.contains(query) | Post.content.contains(query))
    ).all()

    return render_template('search/results.html', posts=posts, query=query)


@search_bp.route('/autocomplete')
def autocomplete():
    """Autocomplete search suggestions."""
    from .app import Post

    query = request.args.get('q', '')
    if len(query) < 2:
        return jsonify([])

    posts = Post.query.filter(
        Post.published == True,
        Post.title.contains(query)
    ).limit(10).all()

    suggestions = [
        {
            'title': post.title,
            'url': url_for('blog.post_detail', slug=post.slug)
        }
        for post in posts
    ]

    return jsonify(suggestions)


# Tags Blueprint
tags_bp = Blueprint('tags', __name__, url_prefix='/tags')


@tags_bp.route('/')
def list_tags():
    """List all tags."""
    from .app import Tag

    tags = Tag.query.all()
    return render_template('tags/list.html', tags=tags)


@tags_bp.route('/<int:tag_id>')
def tag_posts(tag_id):
    """View posts by tag."""
    from .app import Tag

    tag = Tag.query.get_or_404(tag_id)
    return render_template('tags/posts.html', tag=tag, posts=tag.posts)


# Helper function to register all blueprints
def register_blueprints(app):
    """Register all blueprints with the Flask app."""
    app.register_blueprint(api_bp)
    app.register_blueprint(blog_bp)
    app.register_blueprint(admin_bp)
    app.register_blueprint(auth_bp)
    app.register_blueprint(search_bp)
    app.register_blueprint(tags_bp)
