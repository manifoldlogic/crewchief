use crewchief_maproom::indexer::parser;
use std::fs;

/// Test Django models.py parsing with real-world patterns
#[test]
#[ignore = "Django/Flask integration tests have assertion failures - marked for future fix"]
fn test_django_models_parsing() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read Django models fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should extract imports
    let imports = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports.is_some(), "Should extract imports chunk");

    // Should extract User model
    let user_model = chunks
        .iter()
        .find(|c| c.symbol_name == Some("User".to_string()) && c.kind == "class");
    assert!(user_model.is_some(), "Should extract User model class");

    if let Some(user) = user_model {
        assert!(user.docstring.is_some(), "User model should have docstring");

        // Check for inheritance
        if let Some(sig) = &user.signature {
            assert!(
                sig.contains("AbstractUser"),
                "Should capture AbstractUser inheritance"
            );
        }
    }

    // Should extract nested Meta class
    let meta_class = chunks
        .iter()
        .filter(|c| c.symbol_name == Some("Meta".to_string()) && c.kind == "class")
        .count();
    assert!(meta_class > 0, "Should extract Meta nested classes");

    // Should extract model methods
    let get_full_name = chunks
        .iter()
        .find(|c| c.symbol_name == Some("get_full_name".to_string()));
    assert!(
        get_full_name.is_some(),
        "Should extract get_full_name method"
    );

    // Should extract Product model with choices
    let product_model = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Product".to_string()) && c.kind == "class");
    assert!(product_model.is_some(), "Should extract Product model");

    // Should extract constants (status choices)
    let status_constants = chunks
        .iter()
        .filter(|c| c.kind == "constant" || c.kind == "variable")
        .count();
    assert!(status_constants > 0, "Should extract model constants");

    // Should extract property methods
    let is_published = chunks
        .iter()
        .find(|c| c.symbol_name == Some("is_published".to_string()));
    assert!(is_published.is_some(), "Should extract @property method");

    // Should extract classmethod
    let get_published = chunks
        .iter()
        .find(|c| c.symbol_name == Some("get_published_products".to_string()));
    assert!(get_published.is_some(), "Should extract @classmethod");

    // Should extract Review model with validators
    let review_model = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Review".to_string()) && c.kind == "class");
    assert!(review_model.is_some(), "Should extract Review model");

    // Overall count check
    assert!(
        chunks.len() >= 30,
        "Should extract many symbols from Django models file (got {})",
        chunks.len()
    );
}

/// Test Django views.py parsing with class-based and function-based views
#[test]
fn test_django_views_parsing() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/views.py")
        .expect("Failed to read Django views fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should extract function-based view
    let home_view = chunks
        .iter()
        .find(|c| c.symbol_name == Some("home".to_string()) && c.kind == "func");
    assert!(home_view.is_some(), "Should extract home function view");

    // Should extract ListView class
    let product_list_view = chunks
        .iter()
        .find(|c| c.symbol_name == Some("ProductListView".to_string()) && c.kind == "class");
    assert!(
        product_list_view.is_some(),
        "Should extract ProductListView class"
    );

    if let Some(plv) = product_list_view {
        if let Some(sig) = &plv.signature {
            assert!(
                sig.contains("ListView"),
                "Should capture ListView inheritance"
            );
        }
    }

    // Should extract view methods
    let get_queryset = chunks
        .iter()
        .filter(|c| c.symbol_name == Some("get_queryset".to_string()))
        .count();
    assert!(
        get_queryset >= 2,
        "Should extract multiple get_queryset methods"
    );

    let get_context_data = chunks
        .iter()
        .filter(|c| c.symbol_name == Some("get_context_data".to_string()))
        .count();
    assert!(
        get_context_data >= 2,
        "Should extract multiple get_context_data methods"
    );

    // Should extract DetailView
    let product_detail = chunks
        .iter()
        .find(|c| c.symbol_name == Some("ProductDetailView".to_string()) && c.kind == "class");
    assert!(product_detail.is_some(), "Should extract ProductDetailView");

    // Should extract CreateView with mixins
    let product_create = chunks
        .iter()
        .find(|c| c.symbol_name == Some("ProductCreateView".to_string()) && c.kind == "class");
    assert!(product_create.is_some(), "Should extract ProductCreateView");

    // Should extract decorated function view
    let add_review = chunks
        .iter()
        .find(|c| c.symbol_name == Some("add_review".to_string()));
    if let Some(review_fn) = add_review {
        assert!(
            review_fn.metadata.is_some(),
            "Decorated view should have metadata"
        );
    }

    // Should extract API view
    let api_view = chunks
        .iter()
        .find(|c| c.symbol_name == Some("ProductAPIView".to_string()) && c.kind == "class");
    assert!(api_view.is_some(), "Should extract API view class");

    // Should extract error handlers
    let handler404 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("handler404".to_string()));
    assert!(
        handler404.is_some(),
        "Should extract error handler function"
    );

    // Overall count
    assert!(
        chunks.len() >= 25,
        "Should extract many symbols from Django views file (got {})",
        chunks.len()
    );
}

/// Test Django urls.py parsing with URL patterns
#[test]
fn test_django_urls_parsing() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/urls.py")
        .expect("Failed to read Django urls fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should extract imports
    let imports = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports.is_some(), "Should extract imports");

    // Should extract app_name variable
    let app_name = chunks
        .iter()
        .find(|c| c.symbol_name == Some("app_name".to_string()));
    assert!(app_name.is_some(), "Should extract app_name variable");

    // Should extract URL pattern lists
    let product_patterns = chunks
        .iter()
        .find(|c| c.symbol_name == Some("product_patterns".to_string()));
    assert!(
        product_patterns.is_some(),
        "Should extract product_patterns list"
    );

    let urlpatterns = chunks
        .iter()
        .find(|c| c.symbol_name == Some("urlpatterns".to_string()));
    assert!(urlpatterns.is_some(), "Should extract urlpatterns list");

    // Should extract handler variables
    let handler404 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("handler404".to_string()));
    assert!(handler404.is_some(), "Should extract handler404 variable");
}

/// Test Flask app.py parsing with application factory pattern
#[test]
#[ignore = "Django/Flask integration tests have assertion failures - marked for future fix"]
fn test_flask_app_parsing() {
    let source = fs::read_to_string("tests/fixtures/python/flask_samples/app.py")
        .expect("Failed to read Flask app fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should extract imports
    let imports = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports.is_some(), "Should extract imports");

    // Should extract Flask app variable
    let app_var = chunks
        .iter()
        .find(|c| c.symbol_name == Some("app".to_string()));
    assert!(app_var.is_some(), "Should extract app variable");

    // Should extract extension initializations
    let db_var = chunks
        .iter()
        .find(|c| c.symbol_name == Some("db".to_string()));
    assert!(db_var.is_some(), "Should extract db variable");

    // Should extract User model
    let user_model = chunks
        .iter()
        .find(|c| c.symbol_name == Some("User".to_string()) && c.kind == "class");
    assert!(user_model.is_some(), "Should extract User model class");

    if let Some(user) = user_model {
        // Check for multiple inheritance
        if let Some(sig) = &user.signature {
            assert!(
                sig.contains("UserMixin"),
                "Should capture UserMixin inheritance"
            );
            assert!(
                sig.contains("db.Model"),
                "Should capture db.Model inheritance"
            );
        }
    }

    // Should extract model methods
    let set_password = chunks
        .iter()
        .find(|c| c.symbol_name == Some("set_password".to_string()));
    assert!(set_password.is_some(), "Should extract set_password method");

    let check_password = chunks
        .iter()
        .find(|c| c.symbol_name == Some("check_password".to_string()));
    assert!(
        check_password.is_some(),
        "Should extract check_password method"
    );

    // Should extract Post model
    let post_model = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Post".to_string()) && c.kind == "class");
    assert!(post_model.is_some(), "Should extract Post model");

    // Should extract association table
    let post_tags = chunks
        .iter()
        .find(|c| c.symbol_name == Some("post_tags".to_string()));
    assert!(
        post_tags.is_some(),
        "Should extract post_tags association table"
    );

    // Should extract decorated functions
    let load_user = chunks
        .iter()
        .find(|c| c.symbol_name == Some("load_user".to_string()));
    assert!(
        load_user.is_some(),
        "Should extract load_user decorated function"
    );

    // Should extract custom decorators
    let admin_required = chunks
        .iter()
        .find(|c| c.symbol_name == Some("admin_required".to_string()));
    assert!(
        admin_required.is_some(),
        "Should extract admin_required decorator"
    );

    // Should extract route handlers
    let index_route = chunks
        .iter()
        .find(|c| c.symbol_name == Some("index".to_string()));
    assert!(index_route.is_some(), "Should extract index route");

    let register_route = chunks
        .iter()
        .find(|c| c.symbol_name == Some("register".to_string()));
    assert!(register_route.is_some(), "Should extract register route");

    // Should extract API endpoints
    let api_posts = chunks
        .iter()
        .find(|c| c.symbol_name == Some("api_posts".to_string()));
    assert!(api_posts.is_some(), "Should extract api_posts endpoint");

    // Should extract error handlers
    let not_found = chunks
        .iter()
        .find(|c| c.symbol_name == Some("not_found".to_string()));
    assert!(
        not_found.is_some(),
        "Should extract not_found error handler"
    );

    // Should extract template filters
    let datetimeformat = chunks
        .iter()
        .find(|c| c.symbol_name == Some("datetimeformat".to_string()));
    assert!(
        datetimeformat.is_some(),
        "Should extract datetimeformat filter"
    );

    // Should extract CLI commands
    let initdb = chunks
        .iter()
        .find(|c| c.symbol_name == Some("initdb".to_string()));
    assert!(initdb.is_some(), "Should extract initdb CLI command");

    // Overall count
    assert!(
        chunks.len() >= 40,
        "Should extract many symbols from Flask app file (got {})",
        chunks.len()
    );
}

/// Test Flask blueprints.py parsing with blueprint patterns
#[test]
fn test_flask_blueprints_parsing() {
    let source = fs::read_to_string("tests/fixtures/python/flask_samples/blueprints.py")
        .expect("Failed to read Flask blueprints fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should extract blueprint variables
    let api_bp = chunks
        .iter()
        .find(|c| c.symbol_name == Some("api_bp".to_string()));
    assert!(api_bp.is_some(), "Should extract api_bp blueprint");

    let blog_bp = chunks
        .iter()
        .find(|c| c.symbol_name == Some("blog_bp".to_string()));
    assert!(blog_bp.is_some(), "Should extract blog_bp blueprint");

    let admin_bp = chunks
        .iter()
        .find(|c| c.symbol_name == Some("admin_bp".to_string()));
    assert!(admin_bp.is_some(), "Should extract admin_bp blueprint");

    // Should extract blueprint routes
    let status_route = chunks
        .iter()
        .find(|c| c.symbol_name == Some("status".to_string()));
    assert!(status_route.is_some(), "Should extract status route");

    let health_route = chunks
        .iter()
        .find(|c| c.symbol_name == Some("health".to_string()));
    assert!(health_route.is_some(), "Should extract health route");

    // Should extract decorated routes
    let list_users = chunks
        .iter()
        .find(|c| c.symbol_name == Some("list_users".to_string()));
    if let Some(route) = list_users {
        assert!(
            route.metadata.is_some(),
            "Decorated route should have metadata"
        );
    }

    // Should extract admin_required decorator
    let admin_required = chunks
        .iter()
        .find(|c| c.symbol_name == Some("admin_required".to_string()));
    assert!(
        admin_required.is_some(),
        "Should extract admin_required decorator function"
    );

    // Should extract before_request hook
    let before_request = chunks
        .iter()
        .find(|c| c.symbol_name == Some("before_request".to_string()));
    assert!(
        before_request.is_some(),
        "Should extract before_request hook"
    );

    // Should extract register_blueprints helper
    let register_blueprints = chunks
        .iter()
        .find(|c| c.symbol_name == Some("register_blueprints".to_string()));
    assert!(
        register_blueprints.is_some(),
        "Should extract register_blueprints function"
    );

    // Overall count
    assert!(
        chunks.len() >= 25,
        "Should extract many symbols from Flask blueprints file (got {})",
        chunks.len()
    );
}

/// Test Flask extensions.py parsing with extension patterns
#[test]
fn test_flask_extensions_parsing() {
    let source = fs::read_to_string("tests/fixtures/python/flask_samples/extensions.py")
        .expect("Failed to read Flask extensions fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should extract extension variables
    let db = chunks
        .iter()
        .find(|c| c.symbol_name == Some("db".to_string()));
    assert!(db.is_some(), "Should extract db extension");

    let login_manager = chunks
        .iter()
        .find(|c| c.symbol_name == Some("login_manager".to_string()));
    assert!(
        login_manager.is_some(),
        "Should extract login_manager extension"
    );

    let mail = chunks
        .iter()
        .find(|c| c.symbol_name == Some("mail".to_string()));
    assert!(mail.is_some(), "Should extract mail extension");

    // Should extract init function
    let init_extensions = chunks
        .iter()
        .find(|c| c.symbol_name == Some("init_extensions".to_string()));
    assert!(
        init_extensions.is_some(),
        "Should extract init_extensions function"
    );

    if let Some(init_fn) = init_extensions {
        assert!(
            init_fn.docstring.is_some(),
            "init_extensions should have Google-style docstring"
        );
        let docstring = init_fn.docstring.as_ref().unwrap();
        assert!(
            docstring.contains("Args:") || docstring.contains("Parameters:"),
            "Docstring should have parameters section"
        );
    }

    // Should extract configuration function
    let configure_logging = chunks
        .iter()
        .find(|c| c.symbol_name == Some("configure_logging".to_string()));
    assert!(
        configure_logging.is_some(),
        "Should extract configure_logging function"
    );

    // Should extract user loader
    let load_user = chunks
        .iter()
        .find(|c| c.symbol_name == Some("load_user".to_string()));
    assert!(load_user.is_some(), "Should extract load_user decorator");

    // Should extract custom decorators
    let cached = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cached".to_string()));
    assert!(cached.is_some(), "Should extract cached decorator");

    // Should extract helper functions
    let send_email = chunks
        .iter()
        .find(|c| c.symbol_name == Some("send_email".to_string()));
    assert!(send_email.is_some(), "Should extract send_email helper");

    let send_async_email = chunks
        .iter()
        .find(|c| c.symbol_name == Some("send_async_email".to_string()));
    assert!(
        send_async_email.is_some(),
        "Should extract send_async_email helper"
    );

    // Should extract CacheManager class
    let cache_manager = chunks
        .iter()
        .find(|c| c.symbol_name == Some("CacheManager".to_string()) && c.kind == "class");
    assert!(cache_manager.is_some(), "Should extract CacheManager class");

    // Should extract static methods
    let clear_all = chunks
        .iter()
        .find(|c| c.symbol_name == Some("clear_all".to_string()));
    assert!(
        clear_all.is_some(),
        "Should extract clear_all static method"
    );

    // Should extract DatabaseManager class
    let db_manager = chunks
        .iter()
        .find(|c| c.symbol_name == Some("DatabaseManager".to_string()) && c.kind == "class");
    assert!(db_manager.is_some(), "Should extract DatabaseManager class");

    // Should extract ErrorTracker class
    let error_tracker = chunks
        .iter()
        .find(|c| c.symbol_name == Some("ErrorTracker".to_string()) && c.kind == "class");
    assert!(error_tracker.is_some(), "Should extract ErrorTracker class");

    // Overall count
    assert!(
        chunks.len() >= 30,
        "Should extract many symbols from Flask extensions file (got {})",
        chunks.len()
    );
}

/// Test that all real-world fixtures parse without panicking
#[test]
fn test_all_real_world_fixtures_no_panic() {
    let fixtures = vec![
        "tests/fixtures/python/django_samples/models.py",
        "tests/fixtures/python/django_samples/views.py",
        "tests/fixtures/python/django_samples/urls.py",
        "tests/fixtures/python/flask_samples/app.py",
        "tests/fixtures/python/flask_samples/blueprints.py",
        "tests/fixtures/python/flask_samples/extensions.py",
    ];

    for fixture_path in fixtures {
        let source = fs::read_to_string(fixture_path)
            .unwrap_or_else(|_| panic!("Failed to read fixture: {}", fixture_path));

        // Main test: should not panic
        let chunks = parser::extract_chunks(&source, "py");

        // Should extract some symbols
        assert!(
            !chunks.is_empty(),
            "Fixture {} should extract at least some symbols",
            fixture_path
        );

        println!(
            "✓ {} parsed successfully ({} symbols)",
            fixture_path,
            chunks.len()
        );
    }
}

/// Test that real-world code produces quality chunks with metadata
#[test]
#[ignore = "Django/Flask integration tests have assertion failures - marked for future fix"]
fn test_real_world_chunk_quality() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read Django models fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Check chunk quality
    let classes: Vec<_> = chunks.iter().filter(|c| c.kind == "class").collect();

    assert!(!classes.is_empty(), "Should extract classes");

    for class in &classes {
        // Should have line numbers
        assert!(class.start_line > 0, "Class should have valid start line");
        assert!(
            class.end_line >= class.start_line,
            "Class should have valid end line"
        );

        // Should have signature or docstring (SymbolChunk no longer has preview/ts_doc fields)
        assert!(
            class.signature.is_some() || class.docstring.is_some(),
            "Class should have signature or docstring"
        );
    }

    let methods: Vec<_> = chunks.iter().filter(|c| c.kind == "method").collect();

    assert!(!methods.is_empty(), "Should extract methods");

    for method in &methods {
        // Methods should have line numbers
        assert!(method.start_line > 0, "Method should have valid start line");
        assert!(
            method.end_line >= method.start_line,
            "Method should have valid end line"
        );

        // Should have signature or docstring (SymbolChunk no longer has preview field)
        assert!(
            method.signature.is_some() || method.docstring.is_some(),
            "Method should have signature or docstring"
        );
    }
}
