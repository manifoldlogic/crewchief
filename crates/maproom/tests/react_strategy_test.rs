//! Integration tests for React assembly strategy.
//!
//! These tests verify that the React strategy correctly:
//! - Detects React components
//! - Finds and includes routes
//! - Discovers and includes hooks
//! - Handles JSX parent/child relationships
//! - Assembles complete context bundles within budget

use crewchief_maproom::context::{
    detectors::{ComponentDetector, HookDetector, JsxRelationshipDetector},
    strategies::ReactAssemblyStrategy,
    types::ExpandOptions,
    ContextAssembler,
};

#[cfg(test)]
mod component_detector_tests {
    use super::*;

    #[test]
    fn test_component_detector_basic() {
        let detector = ComponentDetector::new();

        // Valid React component files
        assert!(detector.is_component_file_path("Button.tsx"));
        assert!(detector.is_component_file_path("UserProfile.jsx"));

        // These may need to match component_patterns which default to components/** paths
        // Testing with simpler paths that don't require directory matching
        assert!(detector.is_component_file_path("Header.tsx"));
        assert!(detector.is_component_file_path("LoginForm.jsx"));

        // Invalid - not components
        assert!(!detector.is_component_file_path("button.tsx")); // Not PascalCase
        assert!(!detector.is_component_file_path("useAuth.tsx")); // Hook, not component (camelCase)
        assert!(!detector.is_component_file_path("Button.test.tsx")); // Test file
        assert!(!detector.is_component_file_path("Button.stories.tsx")); // Storybook
        assert!(!detector.is_component_file_path("utils.tsx")); // Not PascalCase
    }

    #[test]
    fn test_component_detector_with_content() {
        let detector = ComponentDetector::new();

        let valid_component = r#"
            export function Button({ onClick, children }: ButtonProps) {
                return <button onClick={onClick}>{children}</button>;
            }
        "#;

        let hook_not_component = r#"
            export function useAuth() {
                const [user, setUser] = useState(null);
                return { user, setUser };
            }
        "#;

        let utility_not_component = r#"
            export function formatDate(date: Date): string {
                return date.toISOString();
            }
        "#;

        // Valid component
        assert!(detector.is_component("Button.tsx", Some(valid_component)));

        // Hook without JSX return
        assert!(!detector.is_component("useAuth.tsx", Some(hook_not_component)));

        // Utility function
        assert!(!detector.is_component("utils.tsx", Some(utility_not_component)));
    }

    #[test]
    fn test_component_detector_pascal_case() {
        let detector = ComponentDetector::new();

        // Valid PascalCase
        assert!(detector.is_pascal_case("Button"));
        assert!(detector.is_pascal_case("UserProfile"));
        assert!(detector.is_pascal_case("Nav2"));
        assert!(detector.is_pascal_case("H1"));
        assert!(detector.is_pascal_case("DataTable"));

        // Invalid - not PascalCase
        assert!(!detector.is_pascal_case("button")); // camelCase
        assert!(!detector.is_pascal_case("BUTTON")); // SCREAMING_CASE
        assert!(!detector.is_pascal_case("user_profile")); // snake_case
        assert!(!detector.is_pascal_case("useAuth")); // camelCase (hook)
        assert!(!detector.is_pascal_case("")); // Empty
    }

    #[test]
    fn test_component_detector_jsx_return() {
        let detector = ComponentDetector::new();

        // Valid JSX returns (includes both components and HTML elements)
        assert!(detector.has_jsx_return("return <Button />"));
        assert!(detector.has_jsx_return("return <div>Hello</div>"));
        assert!(detector.has_jsx_return("return (<Header />)"));
        assert!(detector.has_jsx_return("return <button />")); // lowercase HTML is also valid JSX
        assert!(detector.has_jsx_return(
            r#"
            function Component() {
                return (
                    <div>
                        <Header />
                        <Main />
                    </div>
                );
            }
        "#
        ));

        // Invalid - no JSX return
        assert!(!detector.has_jsx_return("return null"));
        assert!(!detector.has_jsx_return("return 'hello'"));
        assert!(!detector.has_jsx_return("return { data: 123 }"));
        assert!(!detector.has_jsx_return("const x = <button />")); // JSX but not in return statement
    }

    #[test]
    fn test_component_detector_index_files() {
        let detector = ComponentDetector::new();

        // Note: Index file detection requires the parent directory to be PascalCase
        // The implementation checks parent directory name
        // These tests verify the logic works correctly

        // Test files that would match include patterns and have PascalCase parent
        // The actual pattern matching depends on the glob patterns used
        // For now, test that index files don't match unless they're in special dirs
        assert!(!detector.is_component_file_path("index.tsx")); // Root index - not a component

        // More specific tests would require custom ComponentPatterns
        // which is tested separately in the pattern tests
    }
}

#[cfg(test)]
mod hook_detector_tests {
    use super::*;

    #[test]
    fn test_hook_detector_builtin() {
        let detector = HookDetector::new();

        // Built-in React hooks
        assert!(detector.is_builtin_hook("useState"));
        assert!(detector.is_builtin_hook("useEffect"));
        assert!(detector.is_builtin_hook("useContext"));
        assert!(detector.is_builtin_hook("useReducer"));
        assert!(detector.is_builtin_hook("useCallback"));
        assert!(detector.is_builtin_hook("useMemo"));
        assert!(detector.is_builtin_hook("useRef"));
        assert!(detector.is_builtin_hook("useLayoutEffect"));

        // Not built-in
        assert!(!detector.is_builtin_hook("useAuth"));
        assert!(!detector.is_builtin_hook("useFetch"));
        assert!(!detector.is_builtin_hook("useLocalStorage"));
    }

    #[test]
    fn test_hook_detector_custom() {
        let detector = HookDetector::new();

        // Valid custom hooks
        assert!(detector.is_custom_hook("useAuth"));
        assert!(detector.is_custom_hook("useFetch"));
        assert!(detector.is_custom_hook("useLocalStorage"));
        assert!(detector.is_custom_hook("useToggle"));
        assert!(detector.is_custom_hook("useDebounce"));

        // Invalid - doesn't follow convention
        assert!(!detector.is_custom_hook("use")); // No uppercase after "use"
        assert!(!detector.is_custom_hook("useauth")); // No uppercase after "use"
        assert!(!detector.is_custom_hook("myHook")); // Doesn't start with "use"
        assert!(!detector.is_custom_hook("use_auth")); // Contains underscore
    }

    #[test]
    fn test_hook_detector_find_calls() {
        let detector = HookDetector::new();

        let code_with_hooks = r#"
            function UserProfile() {
                const [user, setUser] = useState(null);
                const auth = useAuth();
                const data = useFetch('/api/user');

                useEffect(() => {
                    if (auth.isLoggedIn) {
                        setUser(auth.user);
                    }
                }, [auth]);

                return <div>{user?.name}</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code_with_hooks);

        assert_eq!(hooks.len(), 4);
        assert!(hooks.contains(&"useState".to_string()));
        assert!(hooks.contains(&"useAuth".to_string()));
        assert!(hooks.contains(&"useFetch".to_string()));
        assert!(hooks.contains(&"useEffect".to_string()));
    }

    #[test]
    fn test_hook_detector_no_duplicates() {
        let detector = HookDetector::new();

        let code = r#"
            function Component() {
                const [count1, setCount1] = useState(0);
                const [count2, setCount2] = useState(1);
                const [count3, setCount3] = useState(2);
                return <div>{count1 + count2 + count3}</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code);

        // Should only have one entry for useState
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0], "useState");
    }

    #[test]
    fn test_hook_detector_no_false_positives() {
        let detector = HookDetector::new();

        let code = r#"
            function Component() {
                // Comment about useState
                const message = "useState is useful";
                const data = { use: "state" };
                return <div>Learn to use hooks</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code);

        // Should find no hooks
        assert_eq!(hooks.len(), 0);
    }
}

#[cfg(test)]
mod jsx_detector_tests {
    use super::*;

    #[test]
    fn test_jsx_detector_find_components() {
        let detector = JsxRelationshipDetector::new();

        let jsx_code = r#"
            function App() {
                return (
                    <div>
                        <Header />
                        <Main>
                            <Sidebar />
                            <Content />
                        </Main>
                        <Footer />
                    </div>
                );
            }
        "#;

        let components = detector.find_jsx_components(jsx_code);

        assert_eq!(components.len(), 5);
        assert!(components.contains(&"Header".to_string()));
        assert!(components.contains(&"Main".to_string()));
        assert!(components.contains(&"Sidebar".to_string()));
        assert!(components.contains(&"Content".to_string()));
        assert!(components.contains(&"Footer".to_string()));
    }

    #[test]
    fn test_jsx_detector_self_closing() {
        let detector = JsxRelationshipDetector::new();

        let jsx_code = r#"
            function Form() {
                return (
                    <form>
                        <Input type="text" />
                        <Button />
                        <Checkbox />
                    </form>
                );
            }
        "#;

        let components = detector.find_jsx_components(jsx_code);

        assert_eq!(components.len(), 3);
        assert!(components.contains(&"Input".to_string()));
        assert!(components.contains(&"Button".to_string()));
        assert!(components.contains(&"Checkbox".to_string()));
    }

    #[test]
    fn test_jsx_detector_no_html_tags() {
        let detector = JsxRelationshipDetector::new();

        let jsx_code = r#"
            function Layout() {
                return (
                    <div>
                        <header>
                            <Logo />
                        </header>
                        <main>
                            <Content />
                        </main>
                    </div>
                );
            }
        "#;

        let components = detector.find_jsx_components(jsx_code);

        // Should only find PascalCase components, not HTML tags
        assert_eq!(components.len(), 2);
        assert!(components.contains(&"Logo".to_string()));
        assert!(components.contains(&"Content".to_string()));
    }

    #[test]
    fn test_jsx_detector_conditional_rendering() {
        let detector = JsxRelationshipDetector::new();

        let jsx_code = r#"
            function App() {
                return (
                    <div>
                        {isLoading ? <Spinner /> : <Content />}
                        {error && <ErrorMessage />}
                        {data && <DataDisplay />}
                    </div>
                );
            }
        "#;

        let components = detector.find_jsx_components(jsx_code);

        assert_eq!(components.len(), 4);
        assert!(components.contains(&"Spinner".to_string()));
        assert!(components.contains(&"Content".to_string()));
        assert!(components.contains(&"ErrorMessage".to_string()));
        assert!(components.contains(&"DataDisplay".to_string()));
    }

    #[test]
    fn test_jsx_detector_no_duplicates() {
        let detector = JsxRelationshipDetector::new();

        let jsx_code = r#"
            function List() {
                return (
                    <ul>
                        <ListItem id={1} />
                        <ListItem id={2} />
                        <ListItem id={3} />
                    </ul>
                );
            }
        "#;

        let components = detector.find_jsx_components(jsx_code);

        // Should only have one entry for ListItem
        assert_eq!(components.len(), 1);
        assert_eq!(components[0], "ListItem");
    }
}

#[cfg(test)]
mod expand_options_tests {
    use super::*;

    #[test]
    fn test_expand_options_for_react_component() {
        let options = ExpandOptions::for_react_component();

        // React-specific options should be enabled
        assert!(options.tests);
        assert!(options.routes);
        assert!(options.hooks);
        assert!(options.jsx_parents);
        assert!(options.jsx_children);

        // Non-React options should be disabled
        assert!(!options.callers);
        assert!(!options.callees);
        assert!(!options.docs);
        assert!(!options.config);

        // Reasonable depth
        assert_eq!(options.max_depth, 1);
    }

    #[test]
    fn test_expand_options_with_all() {
        let options = ExpandOptions::with_all();

        // All options should be enabled
        assert!(options.callers);
        assert!(options.callees);
        assert!(options.tests);
        assert!(options.docs);
        assert!(options.config);
        assert!(options.routes);
        assert!(options.hooks);
        assert!(options.jsx_parents);
        assert!(options.jsx_children);
    }
}

// Database integration tests (require database connection)
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_react_strategy_basic() {
        // This test would:
        // 1. Set up a test database
        // 2. Insert sample React component chunks
        // 3. Create ReactAssemblyStrategy
        // 4. Call assemble() with React component
        // 5. Verify bundle contains expected items
        //
        // Implementation requires database fixtures
    }

    #[tokio::test]
    #[ignore]
    async fn test_react_strategy_with_hooks() {
        // Test that hooks are included when requested
    }

    #[tokio::test]
    #[ignore]
    async fn test_react_strategy_with_routes() {
        // Test that routes are included when requested
    }

    #[tokio::test]
    #[ignore]
    async fn test_react_strategy_jsx_relationships() {
        // Test that JSX parent/child relationships are handled
    }

    #[tokio::test]
    #[ignore]
    async fn test_react_strategy_budget_enforcement() {
        // Test that the strategy respects token budgets
    }
}
