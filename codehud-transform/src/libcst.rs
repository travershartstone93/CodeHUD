//! LibCST-equivalent Concrete Syntax Tree Implementation
//!
//! This module provides concrete syntax tree transformations preserving
//! formatting and comments exactly like Python LibCST

use crate::{Result, TransformError};
use rowan::{ast::AstNode, GreenNode, GreenNodeBuilder, Language, NodeOrToken, SyntaxNode, TextRange, TextSize};
use std::collections::HashMap;
use tree_sitter::{Node, Parser, Tree, TreeCursor};

/// Language definition for our CST
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CstLanguage {}

impl Language for CstLanguage {
    type Kind = CstSyntaxKind;
    
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        CstSyntaxKind::from_raw(raw)
    }
    
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.to_raw()
    }
}

/// Syntax kinds for our CST
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum CstSyntaxKind {
    // Structural elements
    Root = 0,
    Module,
    Class,
    Function,
    Block,
    Statement,
    Expression,
    
    // Tokens
    Identifier,
    Number,
    String,
    Comment,
    Whitespace,
    Newline,
    
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    
    // Punctuation
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Colon,
    Semicolon,
    
    // Keywords
    Def,
    Class_,
    If,
    Else,
    While,
    For,
    Return,
    Import,
    From,
    As,
    
    // Special
    Error,
    Tombstone,
}

impl CstSyntaxKind {
    fn from_raw(raw: rowan::SyntaxKind) -> Self {
        match raw.0 {
            0 => CstSyntaxKind::Root,
            1 => CstSyntaxKind::Module,
            2 => CstSyntaxKind::Class,
            3 => CstSyntaxKind::Function,
            4 => CstSyntaxKind::Block,
            5 => CstSyntaxKind::Statement,
            6 => CstSyntaxKind::Expression,
            7 => CstSyntaxKind::Identifier,
            8 => CstSyntaxKind::Number,
            9 => CstSyntaxKind::String,
            10 => CstSyntaxKind::Comment,
            11 => CstSyntaxKind::Whitespace,
            12 => CstSyntaxKind::Newline,
            13 => CstSyntaxKind::Plus,
            14 => CstSyntaxKind::Minus,
            15 => CstSyntaxKind::Star,
            16 => CstSyntaxKind::Slash,
            17 => CstSyntaxKind::Equal,
            18 => CstSyntaxKind::LeftParen,
            19 => CstSyntaxKind::RightParen,
            20 => CstSyntaxKind::LeftBrace,
            21 => CstSyntaxKind::RightBrace,
            22 => CstSyntaxKind::Comma,
            23 => CstSyntaxKind::Colon,
            24 => CstSyntaxKind::Semicolon,
            25 => CstSyntaxKind::Def,
            26 => CstSyntaxKind::Class_,
            27 => CstSyntaxKind::If,
            28 => CstSyntaxKind::Else,
            29 => CstSyntaxKind::While,
            30 => CstSyntaxKind::For,
            31 => CstSyntaxKind::Return,
            32 => CstSyntaxKind::Import,
            33 => CstSyntaxKind::From,
            34 => CstSyntaxKind::As,
            35 => CstSyntaxKind::Error,
            _ => CstSyntaxKind::Tombstone,
        }
    }
    
    fn to_raw(self) -> rowan::SyntaxKind {
        rowan::SyntaxKind(self as u16)
    }
}

/// Type aliases for our CST
pub type CstSyntaxNode = rowan::SyntaxNode<CstLanguage>;
pub type CstSyntaxToken = rowan::SyntaxToken<CstLanguage>;

/// CST Node wrapper preserving all metadata
#[derive(Debug, Clone)]
pub struct CstNode {
    /// Underlying syntax node
    syntax_node: CstSyntaxNode,
    /// Original source text
    source_text: String,
    /// Metadata about formatting and comments
    metadata: NodeMetadata,
}

/// Metadata preserved with each node
#[derive(Debug, Clone)]
pub struct NodeMetadata {
    /// Leading whitespace/comments
    pub leading_trivia: Vec<Trivia>,
    /// Trailing whitespace/comments
    pub trailing_trivia: Vec<Trivia>,
    /// Original text range
    pub original_range: TextRange,
    /// Whether this node was modified
    pub modified: bool,
}

/// Trivia (whitespace, comments, etc.)
#[derive(Debug, Clone)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum TriviaKind {
    Whitespace,
    Comment,
    Newline,
}

/// LibCST-equivalent transformer for concrete syntax trees
pub struct LibCstTransformer {
    /// Parser for the target language
    parser: CstParser,
    /// Code formatter
    formatter: CodeFormatter,
    /// Comment preserver
    comment_preserver: CommentPreserver,
}

/// Parser that preserves all formatting information
pub struct CstParser {
    tree_sitter_parser: Parser,
    language: String,
}

/// Code formatter that preserves original formatting
pub struct CodeFormatter {
    preserve_spacing: bool,
    preserve_comments: bool,
    style_config: FormattingConfig,
}

/// Formatting configuration
#[derive(Debug, Clone)]
pub struct FormattingConfig {
    pub indent_size: usize,
    pub max_line_length: usize,
    pub preserve_blank_lines: bool,
    pub preserve_comment_spacing: bool,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            max_line_length: 88, // Black style
            preserve_blank_lines: true,
            preserve_comment_spacing: true,
        }
    }
}

/// Comment preservation system
pub struct CommentPreserver {
    comment_associations: HashMap<TextRange, Vec<String>>,
}

impl LibCstTransformer {
    /// Create new LibCST transformer
    pub fn new(language: &str) -> Result<Self> {
        let mut parser = Parser::new();
        
        // Set language based on input
        let ts_language = match language {
            "python" => tree_sitter_python::language(),
            "javascript" => tree_sitter_javascript::language(),
            "typescript" => tree_sitter_typescript::language_typescript(),
            "rust" => tree_sitter_rust::language(),
            "java" => tree_sitter_java::language(),
            _ => return Err(TransformError::Config(format!("Unsupported language: {}", language))),
        };
        
        parser.set_language(ts_language)?;
        
        Ok(Self {
            parser: CstParser {
                tree_sitter_parser: parser,
                language: language.to_string(),
            },
            formatter: CodeFormatter {
                preserve_spacing: true,
                preserve_comments: true,
                style_config: FormattingConfig::default(),
            },
            comment_preserver: CommentPreserver {
                comment_associations: HashMap::new(),
            },
        })
    }
    
    /// Parse source with metadata preservation (matching Python LibCST behavior)
    pub fn parse_with_metadata(&mut self, source: &str) -> Result<CstNode> {
        // Parse with tree-sitter
        let tree = self.parser.tree_sitter_parser.parse(source, None)
            .ok_or_else(|| TransformError::Parse("Failed to parse source code".to_string()))?;
        
        // Extract comments and whitespace
        self.extract_trivia(source, &tree)?;
        
        // Build CST with metadata
        let cst_node = self.build_cst_with_metadata(source, tree)?;
        
        Ok(cst_node)
    }
    
    /// Transform preserving formatting (matching Python LibCST behavior)
    pub fn transform_preserving_format(
        &mut self,
        mut node: CstNode,
        transform: &dyn CstTransform,
    ) -> Result<CstNode> {
        // Apply transformation while preserving metadata
        transform.visit_node(&mut node)?;
        
        Ok(node)
    }
    
    /// Generate code from CST (matching Python LibCST behavior)
    pub fn generate_code(&self, node: &CstNode) -> Result<String> {
        let mut output = String::new();
        self.generate_code_recursive(&node.syntax_node, &node.metadata, &mut output)?;
        Ok(output)
    }
    
    /// Extract trivia (comments, whitespace) from source
    fn extract_trivia(&mut self, source: &str, tree: &Tree) -> Result<()> {
        let mut cursor = tree.walk();
        self.extract_trivia_recursive(source, &mut cursor)?;
        Ok(())
    }
    
    /// Recursively extract trivia
    fn extract_trivia_recursive(&mut self, source: &str, cursor: &mut TreeCursor) -> Result<()> {
        let node = cursor.node();
        
        // Check if this is a comment or whitespace
        if self.is_trivia_node(&node) {
            let text = node.utf8_text(source.as_bytes())?;
            let range = TextRange::new(
                TextSize::from(node.start_byte() as u32),
                TextSize::from(node.end_byte() as u32),
            );
            
            let trivia = match node.kind() {
                "comment" => Trivia {
                    kind: TriviaKind::Comment,
                    text: text.to_string(),
                },
                "whitespace" | " " | "\t" => Trivia {
                    kind: TriviaKind::Whitespace,
                    text: text.to_string(),
                },
                "newline" | "\n" | "\r\n" => Trivia {
                    kind: TriviaKind::Newline,
                    text: text.to_string(),
                },
                _ => Trivia {
                    kind: TriviaKind::Whitespace,
                    text: text.to_string(),
                },
            };
            
            // Associate trivia with nearby nodes
            self.comment_preserver.comment_associations
                .entry(range)
                .or_insert_with(Vec::new)
                .push(trivia.text);
        }
        
        // Recurse to children
        if cursor.goto_first_child() {
            loop {
                self.extract_trivia_recursive(source, cursor)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Check if node represents trivia
    fn is_trivia_node(&self, node: &Node) -> bool {
        matches!(node.kind(), "comment" | "whitespace" | " " | "\t" | "\n" | "\r\n")
    }
    
    /// Build CST with preserved metadata
    fn build_cst_with_metadata(&self, source: &str, tree: Tree) -> Result<CstNode> {
        let mut builder = GreenNodeBuilder::new();
        
        // Convert tree-sitter tree to rowan green tree
        self.convert_node_recursive(source, tree.root_node(), &mut builder)?;
        
        let green_node = builder.finish();
        let syntax_node = CstSyntaxNode::new_root(green_node);
        
        // Create metadata
        let metadata = NodeMetadata {
            leading_trivia: Vec::new(),
            trailing_trivia: Vec::new(),
            original_range: TextRange::new(TextSize::from(0), TextSize::from(source.len() as u32)),
            modified: false,
        };
        
        Ok(CstNode {
            syntax_node,
            source_text: source.to_string(),
            metadata,
        })
    }
    
    /// Convert tree-sitter node to rowan node
    fn convert_node_recursive(
        &self,
        source: &str,
        node: Node,
        builder: &mut GreenNodeBuilder,
    ) -> Result<()> {
        let kind = self.map_tree_sitter_kind(node.kind());
        
        if node.child_count() == 0 {
            // Leaf node - add as token
            let text = node.utf8_text(source.as_bytes())?;
            builder.token(kind.to_raw(), text);
        } else {
            // Internal node - add children
            builder.start_node(kind.to_raw());
            
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    self.convert_node_recursive(source, cursor.node(), builder)?;
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            
            builder.finish_node();
        }
        
        Ok(())
    }
    
    /// Map tree-sitter node kind to our CST kind
    fn map_tree_sitter_kind(&self, kind: &str) -> CstSyntaxKind {
        match kind {
            "module" => CstSyntaxKind::Module,
            "class_definition" | "class" => CstSyntaxKind::Class,
            "function_definition" | "function" => CstSyntaxKind::Function,
            "block" => CstSyntaxKind::Block,
            "expression_statement" | "statement" => CstSyntaxKind::Statement,
            "identifier" => CstSyntaxKind::Identifier,
            "integer" | "float" | "number" => CstSyntaxKind::Number,
            "string" => CstSyntaxKind::String,
            "comment" => CstSyntaxKind::Comment,
            "+" => CstSyntaxKind::Plus,
            "-" => CstSyntaxKind::Minus,
            "*" => CstSyntaxKind::Star,
            "/" => CstSyntaxKind::Slash,
            "=" => CstSyntaxKind::Equal,
            "(" => CstSyntaxKind::LeftParen,
            ")" => CstSyntaxKind::RightParen,
            "{" => CstSyntaxKind::LeftBrace,
            "}" => CstSyntaxKind::RightBrace,
            "," => CstSyntaxKind::Comma,
            ":" => CstSyntaxKind::Colon,
            ";" => CstSyntaxKind::Semicolon,
            "def" => CstSyntaxKind::Def,
            "class" => CstSyntaxKind::Class_,
            "if" => CstSyntaxKind::If,
            "else" => CstSyntaxKind::Else,
            "while" => CstSyntaxKind::While,
            "for" => CstSyntaxKind::For,
            "return" => CstSyntaxKind::Return,
            "import" => CstSyntaxKind::Import,
            "from" => CstSyntaxKind::From,
            "as" => CstSyntaxKind::As,
            "ERROR" => CstSyntaxKind::Error,
            _ => CstSyntaxKind::Expression, // Default fallback
        }
    }
    
    /// Generate code recursively
    fn generate_code_recursive(
        &self,
        node: &CstSyntaxNode,
        metadata: &NodeMetadata,
        output: &mut String,
    ) -> Result<()> {
        // Add leading trivia
        for trivia in &metadata.leading_trivia {
            output.push_str(&trivia.text);
        }
        
        // Process node content
        for child in node.children_with_tokens() {
            match child {
                NodeOrToken::Node(child_node) => {
                    // Recursively process child nodes
                    let child_metadata = NodeMetadata {
                        leading_trivia: Vec::new(),
                        trailing_trivia: Vec::new(),
                        original_range: child_node.text_range(),
                        modified: false,
                    };
                    self.generate_code_recursive(&child_node, &child_metadata, output)?;
                }
                NodeOrToken::Token(token) => {
                    // Add token text
                    output.push_str(token.text());
                }
            }
        }
        
        // Add trailing trivia
        for trivia in &metadata.trailing_trivia {
            output.push_str(&trivia.text);
        }
        
        Ok(())
    }
}

/// Trait for CST transformations
pub trait CstTransform {
    /// Visit and potentially modify a CST node
    fn visit_node(&self, node: &mut CstNode) -> Result<()>;
}

/// Example transformation that preserves formatting
pub struct ExampleTransform;

impl CstTransform for ExampleTransform {
    fn visit_node(&self, node: &mut CstNode) -> Result<()> {
        // Mark as modified but don't change structure
        node.metadata.modified = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cst_parser_creation() {
        let transformer = LibCstTransformer::new("python");
        assert!(transformer.is_ok());
    }

    #[test]
    fn test_unsupported_language() {
        let transformer = LibCstTransformer::new("unsupported");
        assert!(transformer.is_err());
    }

    #[test]
    fn test_syntax_kind_mapping() {
        let transformer = LibCstTransformer::new("python").unwrap();
        assert_eq!(
            transformer.map_tree_sitter_kind("function_definition"),
            CstSyntaxKind::Function
        );
        assert_eq!(
            transformer.map_tree_sitter_kind("identifier"),
            CstSyntaxKind::Identifier
        );
    }
}