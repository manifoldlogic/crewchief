//! JSX relationship detection.
//!
//! This module provides functionality to detect JSX component relationships:
//! - Parent components that render a target component
//! - Child components rendered by a target component
//! - Props passed between components

use crate::db::Store;
use anyhow::Result;
use regex::Regex;

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
    /// Uses graph traversal to find chunks that call/import this component.
    ///
    /// # Arguments
    /// * `store` - SQLite store
    /// * `target_chunk_id` - Component chunk to find parents for
    /// * `target_symbol_name` - Symbol name of the target component
    ///
    /// # Returns
    /// Vector of parent component chunks
    pub async fn find_parent_components(
        &self,
        store: &(dyn Store + Send + Sync),
        target_chunk_id: i64,
        target_symbol_name: &str,
    ) -> Result<Vec<ComponentUsage>> {
        use crate::db::sqlite::graph::ImportDirection;

        let mut parents = Vec::new();

        // Find chunks that call/reference this component (incoming edges)
        let callers = store.find_callers(target_chunk_id, Some(1)).await?;

        for caller in callers {
            // Get chunk details
            if let Some(chunk) = store.get_chunk_by_id(caller.chunk_id).await? {
                // Check if this chunk contains JSX usage of the target component
                let jsx_components = self.find_jsx_components(&chunk.preview);
                if jsx_components.contains(&target_symbol_name.to_string()) {
                    parents.push(ComponentUsage {
                        id: chunk.id,
                        relpath: chunk.file_path,
                        symbol_name: chunk.symbol_name,
                        kind: chunk.kind,
                        start_line: chunk.start_line,
                        end_line: chunk.end_line,
                        relationship: "parent".to_string(),
                    });
                }
            }
        }

        // Also check imports - chunks that import this component
        let importers = store
            .find_imports(target_chunk_id, ImportDirection::Incoming, Some(1))
            .await?;

        for importer in importers {
            if let Some(chunk) = store.get_chunk_by_id(importer.chunk_id).await? {
                // Check if this chunk uses the target component in JSX
                let jsx_components = self.find_jsx_components(&chunk.preview);
                if jsx_components.contains(&target_symbol_name.to_string()) {
                    // Avoid duplicates
                    if !parents.iter().any(|p| p.id == chunk.id) {
                        parents.push(ComponentUsage {
                            id: chunk.id,
                            relpath: chunk.file_path,
                            symbol_name: chunk.symbol_name,
                            kind: chunk.kind,
                            start_line: chunk.start_line,
                            end_line: chunk.end_line,
                            relationship: "parent".to_string(),
                        });
                    }
                }
            }
        }

        Ok(parents)
    }

    /// Find child components rendered by the target component.
    ///
    /// A child component is one that is rendered in the target's JSX.
    /// Uses graph traversal to find chunks that are called/imported by this component.
    ///
    /// # Arguments
    /// * `store` - SQLite store
    /// * `target_chunk_id` - Component chunk to find children for
    ///
    /// # Returns
    /// Vector of child component chunks
    pub async fn find_child_components(
        &self,
        store: &(dyn Store + Send + Sync),
        target_chunk_id: i64,
    ) -> Result<Vec<ComponentUsage>> {
        use crate::db::sqlite::graph::ImportDirection;

        let mut children = Vec::new();

        // First, get the target chunk to analyze its JSX
        let target_chunk = store.get_chunk_by_id(target_chunk_id).await?;
        if target_chunk.is_none() {
            return Ok(vec![]);
        }
        let target_chunk = target_chunk.unwrap();

        // Find JSX component names used in this component
        let jsx_components = self.find_jsx_components(&target_chunk.preview);

        // Find chunks that this component calls (outgoing edges)
        let callees = store.find_callees(target_chunk_id, Some(1)).await?;

        for callee in callees {
            if let Some(chunk) = store.get_chunk_by_id(callee.chunk_id).await? {
                // Check if this chunk is a component used in the target's JSX
                if let Some(ref symbol_name) = chunk.symbol_name {
                    if jsx_components.contains(symbol_name) {
                        children.push(ComponentUsage {
                            id: chunk.id,
                            relpath: chunk.file_path,
                            symbol_name: chunk.symbol_name,
                            kind: chunk.kind,
                            start_line: chunk.start_line,
                            end_line: chunk.end_line,
                            relationship: "child".to_string(),
                        });
                    }
                }
            }
        }

        // Also check imports - chunks that this component imports
        let imports = store
            .find_imports(target_chunk_id, ImportDirection::Outgoing, Some(1))
            .await?;

        for import in imports {
            if let Some(chunk) = store.get_chunk_by_id(import.chunk_id).await? {
                // Check if this imported chunk is used as a component
                if let Some(ref symbol_name) = chunk.symbol_name {
                    if jsx_components.contains(symbol_name) {
                        // Avoid duplicates
                        if !children.iter().any(|c| c.id == chunk.id) {
                            children.push(ComponentUsage {
                                id: chunk.id,
                                relpath: chunk.file_path,
                                symbol_name: chunk.symbol_name,
                                kind: chunk.kind,
                                start_line: chunk.start_line,
                                end_line: chunk.end_line,
                                relationship: "child".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(children)
    }

    /// Find all JSX relationships for a component.
    ///
    /// Uses tokio::join! to load parents and children concurrently.
    ///
    /// # Arguments
    /// * `store` - SQLite store
    /// * `chunk_id` - Component chunk to analyze
    /// * `symbol_name` - Symbol name of the component
    ///
    /// # Returns
    /// Tuple of (parents, children)
    pub async fn find_all_relationships(
        &self,
        store: &(dyn Store + Send + Sync),
        chunk_id: i64,
        symbol_name: &str,
    ) -> Result<(Vec<ComponentUsage>, Vec<ComponentUsage>)> {
        // Load parents and children in parallel
        let (parents, children) = tokio::join!(
            self.find_parent_components(store, chunk_id, symbol_name),
            self.find_child_components(store, chunk_id)
        );

        Ok((parents.unwrap_or_default(), children.unwrap_or_default()))
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
