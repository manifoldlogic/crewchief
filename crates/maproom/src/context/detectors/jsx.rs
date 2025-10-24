//! JSX relationship detection.
//!
//! This module provides functionality to detect JSX component relationships:
//! - Parent components that render a target component
//! - Child components rendered by a target component
//! - Props passed between components

use anyhow::{Context as AnyhowContext, Result};
use regex::Regex;
use tokio_postgres::Client;

/// JSX component usage information.
#[derive(Debug, Clone)]
pub struct ComponentUsage {
    pub id: i64,
    pub relpath: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub relationship: String, // "parent" or "child"
}

/// Detector for JSX component relationships.
pub struct JsxRelationshipDetector {
    jsx_component_pattern: Regex,
}

impl JsxRelationshipDetector {
    /// Create a new JSX relationship detector.
    pub fn new() -> Self {
        Self {
            // Pattern matches: <ComponentName or </ComponentName
            jsx_component_pattern: Regex::new(r"</?([A-Z][a-zA-Z0-9]*)").unwrap(),
        }
    }

    /// Find component usages in JSX code.
    ///
    /// This extracts component names from JSX syntax.
    ///
    /// # Arguments
    /// * `content` - JSX/TSX code content
    ///
    /// # Returns
    /// Vector of component names found in the JSX
    pub fn find_jsx_components(&self, content: &str) -> Vec<String> {
        let mut components = Vec::new();

        for cap in self.jsx_component_pattern.captures_iter(content) {
            if let Some(component_name) = cap.get(1) {
                let name = component_name.as_str().to_string();
                if !components.contains(&name) {
                    components.push(name);
                }
            }
        }

        components
    }

    /// Find parent components that render the target component.
    ///
    /// A parent component is one that includes JSX rendering the target.
    ///
    /// # Arguments
    /// * `client` - PostgreSQL client
    /// * `target_chunk_id` - Component chunk to find parents for
    /// * `_target_symbol_name` - Symbol name of the target component (reserved for future use)
    ///
    /// # Returns
    /// Vector of parent component chunks
    pub async fn find_parent_components(
        &self,
        client: &Client,
        target_chunk_id: i64,
        _target_symbol_name: &str,
    ) -> Result<Vec<ComponentUsage>> {
        // Strategy 1: Look for render_of edges (if populated)
        // Strategy 2: Search for components that import and use the target

        // First, try to find via edges
        let edge_query = r#"
            SELECT DISTINCT
                c.id,
                f.relpath,
                c.symbol_name,
                c.kind::text,
                c.start_line,
                c.end_line,
                'parent' as relationship
            FROM maproom.chunk_edges ce
            JOIN maproom.chunks c ON c.id = ce.src_chunk_id
            JOIN maproom.files f ON f.id = c.file_id
            WHERE ce.dst_chunk_id = $1
              AND ce.relationship = 'renders'
            ORDER BY f.relpath, c.start_line;
        "#;

        let mut parents: Vec<ComponentUsage> = client
            .query(edge_query, &[&target_chunk_id])
            .await
            .context("Failed to query parent components via edges")?
            .into_iter()
            .map(|row| ComponentUsage {
                id: row.get(0),
                relpath: row.get(1),
                symbol_name: row.get(2),
                kind: row.get(3),
                start_line: row.get(4),
                end_line: row.get(5),
                relationship: row.get(6),
            })
            .collect();

        // Strategy 2: Find components that import the target
        // and have .tsx/.jsx extension (likely render it)
        let import_query = r#"
            SELECT DISTINCT
                c.id,
                f.relpath,
                c.symbol_name,
                c.kind::text,
                c.start_line,
                c.end_line,
                'parent' as relationship
            FROM maproom.chunk_edges ce
            JOIN maproom.chunks c ON c.id = ce.src_chunk_id
            JOIN maproom.files f ON f.id = c.file_id
            WHERE ce.dst_chunk_id = $1
              AND ce.relationship = 'imports'
              AND (f.relpath LIKE '%.tsx' OR f.relpath LIKE '%.jsx')
              AND c.kind IN ('func', 'arrow_func', 'component', 'function')
            ORDER BY f.relpath, c.start_line;
        "#;

        let import_parents: Vec<ComponentUsage> = client
            .query(import_query, &[&target_chunk_id])
            .await
            .context("Failed to query parent components via imports")?
            .into_iter()
            .map(|row| ComponentUsage {
                id: row.get(0),
                relpath: row.get(1),
                symbol_name: row.get(2),
                kind: row.get(3),
                start_line: row.get(4),
                end_line: row.get(5),
                relationship: row.get(6),
            })
            .collect();

        // Merge results, avoiding duplicates
        for import_parent in import_parents {
            if !parents.iter().any(|p| p.id == import_parent.id) {
                parents.push(import_parent);
            }
        }

        Ok(parents)
    }

    /// Find child components rendered by the target component.
    ///
    /// A child component is one that is rendered in the target's JSX.
    ///
    /// # Arguments
    /// * `client` - PostgreSQL client
    /// * `target_chunk_id` - Component chunk to find children for
    ///
    /// # Returns
    /// Vector of child component chunks
    pub async fn find_child_components(
        &self,
        client: &Client,
        target_chunk_id: i64,
    ) -> Result<Vec<ComponentUsage>> {
        // Strategy 1: Look for renders edges (forward)
        let edge_query = r#"
            SELECT DISTINCT
                c.id,
                f.relpath,
                c.symbol_name,
                c.kind::text,
                c.start_line,
                c.end_line,
                'child' as relationship
            FROM maproom.chunk_edges ce
            JOIN maproom.chunks c ON c.id = ce.dst_chunk_id
            JOIN maproom.files f ON f.id = c.file_id
            WHERE ce.src_chunk_id = $1
              AND ce.relationship = 'renders'
            ORDER BY f.relpath, c.start_line;
        "#;

        let mut children: Vec<ComponentUsage> = client
            .query(edge_query, &[&target_chunk_id])
            .await
            .context("Failed to query child components via edges")?
            .into_iter()
            .map(|row| ComponentUsage {
                id: row.get(0),
                relpath: row.get(1),
                symbol_name: row.get(2),
                kind: row.get(3),
                start_line: row.get(4),
                end_line: row.get(5),
                relationship: row.get(6),
            })
            .collect();

        // Strategy 2: Find components that are imported by the target
        // (likely rendered if imported into a component)
        let import_query = r#"
            SELECT DISTINCT
                c.id,
                f.relpath,
                c.symbol_name,
                c.kind::text,
                c.start_line,
                c.end_line,
                'child' as relationship
            FROM maproom.chunk_edges ce
            JOIN maproom.chunks c ON c.id = ce.dst_chunk_id
            JOIN maproom.files f ON f.id = c.file_id
            WHERE ce.src_chunk_id = $1
              AND ce.relationship = 'imports'
              AND (f.relpath LIKE '%.tsx' OR f.relpath LIKE '%.jsx')
              AND c.kind IN ('func', 'arrow_func', 'component', 'function')
            ORDER BY f.relpath, c.start_line;
        "#;

        let import_children: Vec<ComponentUsage> = client
            .query(import_query, &[&target_chunk_id])
            .await
            .context("Failed to query child components via imports")?
            .into_iter()
            .map(|row| ComponentUsage {
                id: row.get(0),
                relpath: row.get(1),
                symbol_name: row.get(2),
                kind: row.get(3),
                start_line: row.get(4),
                end_line: row.get(5),
                relationship: row.get(6),
            })
            .collect();

        // Merge results, avoiding duplicates
        for import_child in import_children {
            if !children.iter().any(|c| c.id == import_child.id) {
                children.push(import_child);
            }
        }

        Ok(children)
    }

    /// Find all JSX relationships for a component.
    ///
    /// # Arguments
    /// * `client` - PostgreSQL client
    /// * `chunk_id` - Component chunk to analyze
    /// * `symbol_name` - Symbol name of the component
    ///
    /// # Returns
    /// Tuple of (parents, children)
    pub async fn find_all_relationships(
        &self,
        client: &Client,
        chunk_id: i64,
        symbol_name: &str,
    ) -> Result<(Vec<ComponentUsage>, Vec<ComponentUsage>)> {
        let (parents, children) = tokio::try_join!(
            self.find_parent_components(client, chunk_id, symbol_name),
            self.find_child_components(client, chunk_id)
        )?;

        Ok((parents, children))
    }
}

impl Default for JsxRelationshipDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_jsx_components_simple() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
            function App() {
                return (
                    <div>
                        <Header />
                        <Main />
                        <Footer />
                    </div>
                );
            }
        "#;

        let components = detector.find_jsx_components(code);

        assert_eq!(components.len(), 3);
        assert!(components.contains(&"Header".to_string()));
        assert!(components.contains(&"Main".to_string()));
        assert!(components.contains(&"Footer".to_string()));
    }

    #[test]
    fn test_find_jsx_components_nested() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
            function Dashboard() {
                return (
                    <Container>
                        <Sidebar>
                            <Nav />
                            <UserProfile />
                        </Sidebar>
                        <Content>
                            <DataTable />
                        </Content>
                    </Container>
                );
            }
        "#;

        let components = detector.find_jsx_components(code);

        assert_eq!(components.len(), 6);
        assert!(components.contains(&"Container".to_string()));
        assert!(components.contains(&"Sidebar".to_string()));
        assert!(components.contains(&"Nav".to_string()));
        assert!(components.contains(&"UserProfile".to_string()));
        assert!(components.contains(&"Content".to_string()));
        assert!(components.contains(&"DataTable".to_string()));
    }

    #[test]
    fn test_find_jsx_components_with_props() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
            function Page() {
                return (
                    <Layout title="Home">
                        <Hero image={logo} />
                        <Section>
                            <Card title="Welcome" />
                        </Section>
                    </Layout>
                );
            }
        "#;

        let components = detector.find_jsx_components(code);

        assert!(components.contains(&"Layout".to_string()));
        assert!(components.contains(&"Hero".to_string()));
        assert!(components.contains(&"Section".to_string()));
        assert!(components.contains(&"Card".to_string()));
    }

    #[test]
    fn test_find_jsx_components_self_closing() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
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

        let components = detector.find_jsx_components(code);

        assert_eq!(components.len(), 3);
        assert!(components.contains(&"Input".to_string()));
        assert!(components.contains(&"Button".to_string()));
        assert!(components.contains(&"Checkbox".to_string()));
    }

    #[test]
    fn test_find_jsx_components_with_closing_tags() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
            function App() {
                return (
                    <Modal>
                        <ModalHeader>Title</ModalHeader>
                        <ModalBody>Content</ModalBody>
                        <ModalFooter>Actions</ModalFooter>
                    </Modal>
                );
            }
        "#;

        let components = detector.find_jsx_components(code);

        // Should deduplicate: Modal, ModalHeader, ModalBody, ModalFooter
        assert_eq!(components.len(), 4);
        assert!(components.contains(&"Modal".to_string()));
        assert!(components.contains(&"ModalHeader".to_string()));
        assert!(components.contains(&"ModalBody".to_string()));
        assert!(components.contains(&"ModalFooter".to_string()));
    }

    #[test]
    fn test_find_jsx_components_no_html_tags() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
            function Layout() {
                return (
                    <div>
                        <header>
                            <nav>
                                <Logo />
                            </nav>
                        </header>
                        <main>
                            <Content />
                        </main>
                    </div>
                );
            }
        "#;

        let components = detector.find_jsx_components(code);

        // Should only find PascalCase components, not HTML tags
        assert_eq!(components.len(), 2);
        assert!(components.contains(&"Logo".to_string()));
        assert!(components.contains(&"Content".to_string()));
    }

    #[test]
    fn test_find_jsx_components_conditional_rendering() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
            function App() {
                return (
                    <div>
                        {isLoading ? <Spinner /> : <Content />}
                        {error && <ErrorMessage />}
                        {data && <DataDisplay data={data} />}
                    </div>
                );
            }
        "#;

        let components = detector.find_jsx_components(code);

        assert_eq!(components.len(), 4);
        assert!(components.contains(&"Spinner".to_string()));
        assert!(components.contains(&"Content".to_string()));
        assert!(components.contains(&"ErrorMessage".to_string()));
        assert!(components.contains(&"DataDisplay".to_string()));
    }

    #[test]
    fn test_find_jsx_components_fragments() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
            function List() {
                return (
                    <>
                        <ListItem />
                        <ListItem />
                        <ListItem />
                    </>
                );
            }
        "#;

        let components = detector.find_jsx_components(code);

        assert_eq!(components.len(), 1);
        assert!(components.contains(&"ListItem".to_string()));
    }

    #[test]
    fn test_find_jsx_components_no_duplicates() {
        let detector = JsxRelationshipDetector::new();

        let code = r#"
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

        let components = detector.find_jsx_components(code);

        // Should only have one entry for ListItem
        assert_eq!(components.len(), 1);
        assert_eq!(components[0], "ListItem");
    }

    // Database tests are in integration tests
    #[tokio::test]
    #[ignore]
    async fn test_find_parent_components() {
        // Integration test - requires database
    }

    #[tokio::test]
    #[ignore]
    async fn test_find_child_components() {
        // Integration test - requires database
    }

    #[tokio::test]
    #[ignore]
    async fn test_find_all_relationships() {
        // Integration test - requires database
    }
}
