//! Graph node definitions for CodeHUD graph analysis
//!
//! Defines the node types used in different graph representations:
//! - CallNode: Function calls in call graphs
//! - ModuleNode: Modules/files in dependency graphs
//! - ClassNode: Classes in inheritance graphs

use serde::{Serialize, Deserialize};

/// Node representing a function in the call graph
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallNode {
    /// Function name
    pub function_name: String,
    /// File path where function is defined
    pub file_path: String,
    /// Line number where function is defined
    pub line_number: usize,
}

impl CallNode {
    /// Create a new call node
    pub fn new(function_name: String, file_path: String, line_number: usize) -> Self {
        Self {
            function_name,
            file_path,
            line_number,
        }
    }

    /// Get the qualified function name (file::function)
    pub fn qualified_name(&self) -> String {
        format!("{}::{}", self.file_path, self.function_name)
    }
}

/// Node representing a module in the dependency graph
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleNode {
    /// Module name
    pub module_name: String,
    /// File path of the module
    pub file_path: String,
    /// Whether this is an external dependency
    pub is_external: bool,
}

impl ModuleNode {
    /// Create a new module node
    pub fn new(module_name: String, file_path: String, is_external: bool) -> Self {
        Self {
            module_name,
            file_path,
            is_external,
        }
    }

    /// Check if this is an internal module
    pub fn is_internal(&self) -> bool {
        !self.is_external
    }

    /// Get module type as string
    pub fn module_type(&self) -> &'static str {
        if self.is_external {
            "external"
        } else {
            "internal"
        }
    }
}

/// Node representing a class in the inheritance graph
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClassNode {
    /// Class name
    pub class_name: String,
    /// File path where class is defined
    pub file_path: String,
    /// Line number where class is defined
    pub line_number: usize,
}

impl ClassNode {
    /// Create a new class node
    pub fn new(class_name: String, file_path: String, line_number: usize) -> Self {
        Self {
            class_name,
            file_path,
            line_number,
        }
    }

    /// Get the qualified class name (file::class)
    pub fn qualified_name(&self) -> String {
        format!("{}::{}", self.file_path, self.class_name)
    }
}

/// Trait for graph nodes to provide common functionality
pub trait GraphNode {
    /// Get the display name for this node
    fn display_name(&self) -> &str;

    /// Get the file path for this node
    fn file_path(&self) -> &str;

    /// Get the line number for this node (if applicable)
    fn line_number(&self) -> Option<usize>;
}

impl GraphNode for CallNode {
    fn display_name(&self) -> &str {
        &self.function_name
    }

    fn file_path(&self) -> &str {
        &self.file_path
    }

    fn line_number(&self) -> Option<usize> {
        Some(self.line_number)
    }
}

impl GraphNode for ModuleNode {
    fn display_name(&self) -> &str {
        &self.module_name
    }

    fn file_path(&self) -> &str {
        &self.file_path
    }

    fn line_number(&self) -> Option<usize> {
        None
    }
}

impl GraphNode for ClassNode {
    fn display_name(&self) -> &str {
        &self.class_name
    }

    fn file_path(&self) -> &str {
        &self.file_path
    }

    fn line_number(&self) -> Option<usize> {
        Some(self.line_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_node_creation() {
        let node = CallNode::new(
            "main".to_string(),
            "src/main.py".to_string(),
            10
        );

        assert_eq!(node.function_name, "main");
        assert_eq!(node.file_path, "src/main.py");
        assert_eq!(node.line_number, 10);
        assert_eq!(node.qualified_name(), "src/main.py::main");
    }

    #[test]
    fn test_module_node_creation() {
        let node = ModuleNode::new(
            "utils".to_string(),
            "src/utils.py".to_string(),
            false
        );

        assert_eq!(node.module_name, "utils");
        assert_eq!(node.file_path, "src/utils.py");
        assert!(!node.is_external);
        assert!(node.is_internal());
        assert_eq!(node.module_type(), "internal");
    }

    #[test]
    fn test_external_module_node() {
        let node = ModuleNode::new(
            "requests".to_string(),
            "".to_string(),
            true
        );

        assert!(node.is_external);
        assert!(!node.is_internal());
        assert_eq!(node.module_type(), "external");
    }

    #[test]
    fn test_class_node_creation() {
        let node = ClassNode::new(
            "MyClass".to_string(),
            "src/classes.py".to_string(),
            25
        );

        assert_eq!(node.class_name, "MyClass");
        assert_eq!(node.file_path, "src/classes.py");
        assert_eq!(node.line_number, 25);
        assert_eq!(node.qualified_name(), "src/classes.py::MyClass");
    }

    #[test]
    fn test_graph_node_trait() {
        let call_node = CallNode::new("func".to_string(), "test.py".to_string(), 5);
        let module_node = ModuleNode::new("mod".to_string(), "test.py".to_string(), false);
        let class_node = ClassNode::new("Class".to_string(), "test.py".to_string(), 10);

        assert_eq!(call_node.display_name(), "func");
        assert_eq!(call_node.file_path(), "test.py");
        assert_eq!(call_node.line_number(), Some(5));

        assert_eq!(module_node.display_name(), "mod");
        assert_eq!(module_node.file_path(), "test.py");
        assert_eq!(module_node.line_number(), None);

        assert_eq!(class_node.display_name(), "Class");
        assert_eq!(class_node.file_path(), "test.py");
        assert_eq!(class_node.line_number(), Some(10));
    }
}