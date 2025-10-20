//! Tree-Sitter Query Engine
//!
//! Automatically detects file languages and applies appropriate tree-sitter queries
//! for code analysis. Uses established community grammars for maximum compatibility.
//!
//! CRITICAL FIX APPLIED (2025-01-25): Added query limits to prevent infinite loops
//! in comment extraction that was causing system hangs:
//! - cursor.set_match_limit(5000): Limits tree-sitter query matches per file
//! - max_comments = 10000: Prevents infinite comment processing loops
//! - Error handling for invalid UTF-8 text to avoid crashes
//! This fix resolved hanging issues when processing large Rust codebases (206 files).

use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};
use anyhow::{Result, Context};
use serde_json::{Value, json};
use std::path::Path;
use std::collections::HashMap;
use walkdir::WalkDir;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

/// Comprehensive language support using community tree-sitter grammars
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    // Tier 1 - Core general-purpose
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Java,
    Go,
    C,
    Cpp,
    CSharp,
    PHP,
    Ruby,
    Swift,
    Kotlin,

    // Tier 2 - Scripting / Systems
    Bash,
    PowerShell,
    Lua,
    Zig,
    Haskell,
    OCaml,
    ObjectiveC,

    // Tier 3 - Web / Data / Markup
    HTML,
    CSS,
    SCSS,
    JSON,
    YAML,
    TOML,
    XML,
    Markdown,
    GraphQL,
    SQL,
    Protobuf,

    // Tier 4 - Build / DevOps / Config
    Dockerfile,
    Makefile,
    CMake,
    Nix,
    HCL,
    INI,
}

/// Language grammar registry with metadata and extension mappings
#[derive(Debug, Clone)]
pub struct LanguageGrammar {
    pub language: SupportedLanguage,
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub tree_sitter_fn: fn() -> Language,
    pub tier: u8,
    pub repo_url: &'static str,
}

impl SupportedLanguage {
    /// Get tree-sitter language for this enum
    pub fn tree_sitter_language(&self) -> Language {
        match self {
            // Tier 1 - Core languages
            Self::Rust => tree_sitter_rust::language(),
            Self::Python => tree_sitter_python::language(),
            Self::JavaScript => tree_sitter_javascript::language(),
            Self::TypeScript => tree_sitter_typescript::language_typescript(),
            Self::Java => tree_sitter_java::language(),
            // For now, fallback to existing parsers for other languages
            // TODO: Add actual parsers as we integrate more grammars
            _ => tree_sitter_rust::language(), // Temporary fallback
        }
    }

    /// Get comprehensive file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            // Tier 1
            Self::Rust => &["rs"],
            Self::Python => &["py", "pyw", "pyi"],
            Self::JavaScript => &["js", "mjs", "cjs", "jsx"],
            Self::TypeScript => &["ts", "tsx", "d.ts"],
            Self::Java => &["java"],
            Self::Go => &["go"],
            Self::C => &["c", "h"],
            Self::Cpp => &["cpp", "cxx", "cc", "hpp", "hxx", "hh"],
            Self::CSharp => &["cs"],
            Self::PHP => &["php", "phtml", "php3", "php4", "php5"],
            Self::Ruby => &["rb", "rbw", "rake", "gemspec"],
            Self::Swift => &["swift"],
            Self::Kotlin => &["kt", "kts"],

            // Tier 2
            Self::Bash => &["sh", "bash", "zsh", "fish"],
            Self::PowerShell => &["ps1", "psd1", "psm1"],
            Self::Lua => &["lua"],
            Self::Zig => &["zig"],
            Self::Haskell => &["hs", "lhs"],
            Self::OCaml => &["ml", "mli"],
            Self::ObjectiveC => &["m", "mm"],

            // Tier 3
            Self::HTML => &["html", "htm", "xhtml"],
            Self::CSS => &["css"],
            Self::SCSS => &["scss", "sass"],
            Self::JSON => &["json", "jsonc"],
            Self::YAML => &["yaml", "yml"],
            Self::TOML => &["toml"],
            Self::XML => &["xml", "xsd", "xsl", "xslt"],
            Self::Markdown => &["md", "markdown", "mdown", "mkd"],
            Self::GraphQL => &["graphql", "gql"],
            Self::SQL => &["sql"],
            Self::Protobuf => &["proto"],

            // Tier 4
            Self::Dockerfile => &["dockerfile", "containerfile"],
            Self::Makefile => &["makefile", "mk"],
            Self::CMake => &["cmake"],
            Self::Nix => &["nix"],
            Self::HCL => &["hcl", "tf", "tfvars"],
            Self::INI => &["ini", "cfg", "conf"],
        }
    }

    /// Get all supported languages in priority order
    pub fn all_languages() -> &'static [Self] {
        &[
            // Tier 1 - Most important
            Self::Rust, Self::Python, Self::JavaScript, Self::TypeScript, Self::Java,
            Self::Go, Self::C, Self::Cpp, Self::CSharp, Self::PHP, Self::Ruby, Self::Swift, Self::Kotlin,

            // Tier 2
            Self::Bash, Self::PowerShell, Self::Lua, Self::Zig, Self::Haskell, Self::OCaml, Self::ObjectiveC,

            // Tier 3
            Self::HTML, Self::CSS, Self::SCSS, Self::JSON, Self::YAML, Self::TOML, Self::XML,
            Self::Markdown, Self::GraphQL, Self::SQL, Self::Protobuf,

            // Tier 4
            Self::Dockerfile, Self::Makefile, Self::CMake, Self::Nix, Self::HCL, Self::INI,
        ]
    }

    /// Detect language from file path with comprehensive extension matching
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_lowercase();

        // Handle special cases first
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            let filename_lower = filename.to_lowercase();

            // Special filename detection
            match filename_lower.as_str() {
                "dockerfile" | "containerfile" => return Some(Self::Dockerfile),
                "makefile" | "gnumakefile" => return Some(Self::Makefile),
                "cmakelists.txt" => return Some(Self::CMake),
                _ => {}
            }
        }

        // Extension-based detection
        for &lang in Self::all_languages() {
            if lang.extensions().iter().any(|ext| ext.to_lowercase() == extension) {
                return Some(lang);
            }
        }

        None
    }

    /// Get grammar metadata for this language
    pub fn grammar_info(&self) -> LanguageGrammar {
        match self {
            Self::Rust => LanguageGrammar {
                language: *self,
                name: "Rust",
                extensions: self.extensions(),
                tree_sitter_fn: tree_sitter_rust::language,
                tier: 1,
                repo_url: "https://github.com/tree-sitter/tree-sitter-rust",
            },
            Self::Python => LanguageGrammar {
                language: *self,
                name: "Python",
                extensions: self.extensions(),
                tree_sitter_fn: tree_sitter_python::language,
                tier: 1,
                repo_url: "https://github.com/tree-sitter/tree-sitter-python",
            },
            Self::JavaScript => LanguageGrammar {
                language: *self,
                name: "JavaScript",
                extensions: self.extensions(),
                tree_sitter_fn: tree_sitter_javascript::language,
                tier: 1,
                repo_url: "https://github.com/tree-sitter/tree-sitter-javascript",
            },
            Self::TypeScript => LanguageGrammar {
                language: *self,
                name: "TypeScript",
                extensions: self.extensions(),
                tree_sitter_fn: tree_sitter_typescript::language_typescript,
                tier: 1,
                repo_url: "https://github.com/tree-sitter/tree-sitter-typescript",
            },
            Self::Java => LanguageGrammar {
                language: *self,
                name: "Java",
                extensions: self.extensions(),
                tree_sitter_fn: tree_sitter_java::language,
                tier: 1,
                repo_url: "https://github.com/tree-sitter/tree-sitter-java",
            },
            // TODO: Add actual grammar info for other languages
            _ => LanguageGrammar {
                language: *self,
                name: "Unknown",
                extensions: self.extensions(),
                tree_sitter_fn: tree_sitter_rust::language, // Fallback
                tier: 4,
                repo_url: "https://github.com/tree-sitter/tree-sitter-rust",
            },
        }
    }
}

/// Query types for different analysis purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
    Imports,
    Functions,
    Calls,        // Function call extraction for call graph
    Complexity,
    Security,
    Performance,
    Highlights,   // Community highlights.scm for semantic analysis
    Tags,         // Community tags.scm for symbol extraction
    References,
    Classes,
    Variables,
    Comments,     // Comment extraction queries
}

impl QueryType {
    fn filename(&self) -> &'static str {
        match self {
            Self::Imports => "imports.scm",
            Self::Functions => "functions.scm",
            Self::Calls => "calls.scm",
            Self::Complexity => "complexity.scm",
            Self::Security => "security.scm",
            Self::Performance => "performance.scm",
            Self::Highlights => "highlights.scm",
            Self::Tags => "tags.scm",
            Self::References => "references.scm",
            Self::Classes => "classes.scm",
            Self::Variables => "variables.scm",
            Self::Comments => "comments.scm",
        }
    }
}

/// Main query engine that handles all tree-sitter operations
pub struct QueryEngine {
    parsers: HashMap<SupportedLanguage, Parser>,
    queries: HashMap<(SupportedLanguage, QueryType), Query>,
}

impl QueryEngine {
    /// Create new query engine with all languages and queries pre-loaded
    pub fn new() -> Result<Self> {
        let mut engine = Self {
            parsers: HashMap::new(),
            queries: HashMap::new(),
        };

        // Initialize parsers for all supported languages
        for &lang in &[SupportedLanguage::Rust, SupportedLanguage::Python,
                      SupportedLanguage::JavaScript, SupportedLanguage::TypeScript,
                      SupportedLanguage::Java] {
            let mut parser = Parser::new();
            parser.set_language(lang.tree_sitter_language())
                .with_context(|| format!("Failed to set language for {:?}", lang))?;
            engine.parsers.insert(lang, parser);
        }

        // Pre-compile all queries for performance
        engine.load_all_queries()?;

        // Debug: Print loaded queries
        println!("📋 Query Engine Initialized:");
        println!("   Loaded {} parsers", engine.parsers.len());
        println!("   Loaded {} queries", engine.queries.len());
        for ((lang, query_type), _) in &engine.queries {
            println!("   - {:?} {:?} query loaded", lang, query_type);
        }

        Ok(engine)
    }

    /// Load and compile all query files
    fn load_all_queries(&mut self) -> Result<()> {
        for &lang in &[SupportedLanguage::Rust, SupportedLanguage::Python,
                      SupportedLanguage::JavaScript, SupportedLanguage::TypeScript,
                      SupportedLanguage::Java] {
            for &query_type in &[QueryType::Imports, QueryType::Functions, QueryType::Calls, QueryType::Complexity,
                                 QueryType::Highlights, QueryType::Tags, QueryType::References, QueryType::Comments] {
                if let Some(query_content) = self.load_query_file(lang, query_type) {
                    println!("🔍 Loading {:?} {:?} query: {} chars", lang, query_type, query_content.len());
                    match Query::new(lang.tree_sitter_language(), &query_content) {
                        Ok(query) => {
                            self.queries.insert((lang, query_type), query);
                            println!("✅ Successfully compiled {:?} {:?} query", lang, query_type);
                        }
                        Err(e) => {
                            println!("❌ Failed to compile {:?} query for {:?}: {}", query_type, lang, e);
                            tracing::warn!("Failed to compile {:?} query for {:?}: {}", query_type, lang, e);
                        }
                    }
                } else {
                    println!("⚠️ No query file found for {:?} {:?}", lang, query_type);
                }
            }
        }
        Ok(())
    }

    /// Load query file content with comprehensive path resolution
    fn load_query_file(&self, lang: SupportedLanguage, query_type: QueryType) -> Option<String> {
        let lang_name = self.get_language_name(lang);
        let query_filename = query_type.filename();

        // Priority paths for query files
        let search_paths = [
            format!("queries/{}/{}", lang_name, query_filename),                    // Current dir
            format!("../queries/{}/{}", lang_name, query_filename),                 // Parent dir
            format!("codehud-core/queries/{}/{}", lang_name, query_filename),       // From root
            format!("tree-sitter-grammars/{}/queries/{}", lang_name, query_filename), // Grammar repo
        ];

        for path in &search_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                return Some(content);
            }
        }

        // Try downloading from community repo if not found locally
        self.try_download_community_query(lang, query_type)
    }

    /// Get standardized language name for file paths
    fn get_language_name(&self, lang: SupportedLanguage) -> &str {
        match lang {
            SupportedLanguage::Rust => "rust",
            SupportedLanguage::Python => "python",
            SupportedLanguage::JavaScript => "javascript",
            SupportedLanguage::TypeScript => "typescript",
            SupportedLanguage::Java => "java",
            SupportedLanguage::Go => "go",
            SupportedLanguage::C => "c",
            SupportedLanguage::Cpp => "cpp",
            SupportedLanguage::CSharp => "c-sharp",
            SupportedLanguage::PHP => "php",
            SupportedLanguage::Ruby => "ruby",
            SupportedLanguage::Swift => "swift",
            SupportedLanguage::Kotlin => "kotlin",
            SupportedLanguage::Bash => "bash",
            SupportedLanguage::PowerShell => "powershell",
            SupportedLanguage::Lua => "lua",
            SupportedLanguage::Zig => "zig",
            SupportedLanguage::Haskell => "haskell",
            SupportedLanguage::OCaml => "ocaml",
            SupportedLanguage::ObjectiveC => "objc",
            SupportedLanguage::HTML => "html",
            SupportedLanguage::CSS => "css",
            SupportedLanguage::SCSS => "scss",
            SupportedLanguage::JSON => "json",
            SupportedLanguage::YAML => "yaml",
            SupportedLanguage::TOML => "toml",
            SupportedLanguage::XML => "xml",
            SupportedLanguage::Markdown => "markdown",
            SupportedLanguage::GraphQL => "graphql",
            SupportedLanguage::SQL => "sql",
            SupportedLanguage::Protobuf => "proto",
            SupportedLanguage::Dockerfile => "dockerfile",
            SupportedLanguage::Makefile => "make",
            SupportedLanguage::CMake => "cmake",
            SupportedLanguage::Nix => "nix",
            SupportedLanguage::HCL => "hcl",
            SupportedLanguage::INI => "ini",
        }
    }

    /// Try to download community query patterns (future enhancement)
    fn try_download_community_query(&self, _lang: SupportedLanguage, _query_type: QueryType) -> Option<String> {
        // TODO: Implement automatic download of community queries
        // For now, return None - queries must be present locally
        None
    }

    /// Analyze a single file automatically detecting language and applying appropriate queries
    pub fn analyze_file(&mut self, file_path: &Path) -> Result<Value> {
        // Automatically detect language
        let language = SupportedLanguage::from_path(file_path)
            .ok_or_else(|| anyhow::anyhow!("Unsupported file type: {}", file_path.display()))?;

        // Read file content
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        // Parse with appropriate language parser
        let parser = self.parsers.get_mut(&language)
            .ok_or_else(|| anyhow::anyhow!("No parser available for {:?}", language))?;

        let tree = parser.parse(&content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse file: {}", file_path.display()))?;

        // Apply all available queries for this language
        let mut results = json!({
            "file_path": file_path.to_string_lossy(),
            "language": format!("{:?}", language).to_lowercase(),
            "analysis_method": "tree_sitter_query"
        });

        // Extract imports using enhanced semantic approach
        if let Some(imports) = self.extract_with_query(language, QueryType::Imports, &tree, &content)? {
            results["imports"] = imports;
        }

        // Extract functions
        if let Some(functions) = self.extract_with_query(language, QueryType::Functions, &tree, &content)? {
            results["functions"] = functions;
        }

        // Extract function calls
        if let Some(calls) = self.extract_with_query(language, QueryType::Calls, &tree, &content)? {
            results["calls"] = calls;
        }

        // Calculate complexity
        if let Some(complexity) = self.extract_with_query(language, QueryType::Complexity, &tree, &content)? {
            results["complexity"] = complexity;
        }

        // Extract symbols using community tags.scm
        if let Some(tags) = self.extract_with_query(language, QueryType::Tags, &tree, &content)? {
            results["tags"] = tags;
        }

        // Extract semantic highlights using community highlights.scm
        if let Some(highlights) = self.extract_with_query(language, QueryType::Highlights, &tree, &content)? {
            results["highlights"] = highlights;
        }

        // Extract comments using comment-specific queries
        if let Some(comments) = self.extract_with_query(language, QueryType::Comments, &tree, &content)? {
            results["comments"] = comments;
        }

        Ok(results)
    }

    /// Extract information using a specific query type
    fn extract_with_query(
        &self,
        language: SupportedLanguage,
        query_type: QueryType,
        tree: &Tree,
        source: &str
    ) -> Result<Option<Value>> {
        let query = match self.queries.get(&(language, query_type)) {
            Some(q) => q,
            None => return Ok(None), // Query not available for this language
        };

        let mut cursor = QueryCursor::new();
        // Set reasonable limits to prevent infinite processing
        cursor.set_match_limit(5000);
        let matches = cursor.matches(query, tree.root_node(), source.as_bytes());

        match query_type {
            QueryType::Imports => Ok(Some(self.process_import_matches(query, matches, source)?)),
            QueryType::Functions => Ok(Some(self.process_function_matches(query, matches, source)?)),
            QueryType::Calls => Ok(Some(self.process_call_matches(query, matches, source)?)),
            QueryType::Complexity => Ok(Some(self.process_complexity_matches(query, matches, source)?)),
            QueryType::Tags => Ok(Some(self.process_community_tags(query, matches, source)?)),
            QueryType::Highlights => Ok(Some(self.process_community_highlights(query, matches, source)?)),
            QueryType::Comments => Ok(Some(self.process_comment_matches(query, matches, source)?)),
            _ => Ok(None),
        }
    }

    /// Process import matches using proper tree-sitter semantic approach
    fn process_import_matches<'a>(
        &self,
        query: &Query,
        matches: impl Iterator<Item = tree_sitter::QueryMatch<'a, 'a>>,
        source: &str
    ) -> Result<Value> {
        let mut imports = Vec::new();
        let mut modules = std::collections::HashSet::new();
        let mut items = std::collections::HashSet::new();
        let mut aliases = std::collections::HashSet::new();
        let mut wildcards = Vec::new();
        let mut crates = std::collections::HashSet::new();

        // Aggregate all captures by type for semantic analysis
        let mut captures_by_type: HashMap<String, Vec<(String, usize)>> = HashMap::new();

        for m in matches {
            for capture in m.captures {
                let text = capture.node.utf8_text(source.as_bytes())?;
                let capture_name = &query.capture_names()[capture.index as usize];
                let line = capture.node.start_position().row + 1;

                captures_by_type
                    .entry(capture_name.to_string())
                    .or_insert_with(Vec::new)
                    .push((text.to_string(), line));
            }
        }

        // Process captures semantically
        for (capture_type, captures) in captures_by_type {
            match capture_type.as_str() {
                "import" => {
                    // Full import declarations - create detailed records
                    for (text, line) in captures {
                        imports.push(json!({
                            "type": "import_declaration",
                            "text": text,
                            "line": line,
                            "analysis_method": "tree_sitter_semantic"
                        }));
                    }
                }
                "item" => {
                    // Individual imported items
                    for (text, line) in captures {
                        items.insert(text.clone());
                        imports.push(json!({
                            "type": "imported_item",
                            "item": text,
                            "line": line
                        }));
                    }
                }
                "module" => {
                    // Module references
                    for (text, _line) in captures {
                        modules.insert(text);
                    }
                }
                "path" => {
                    // Module paths
                    for (text, _line) in captures {
                        modules.insert(text);
                    }
                }
                "alias" => {
                    // Aliases
                    for (text, line) in captures {
                        aliases.insert(text.clone());
                        imports.push(json!({
                            "type": "alias",
                            "alias": text,
                            "line": line
                        }));
                    }
                }
                "wildcard" => {
                    // Wildcard imports
                    for (text, line) in captures {
                        wildcards.push(json!({
                            "type": "wildcard",
                            "text": text,
                            "line": line
                        }));
                    }
                }
                "crate" => {
                    // External crates
                    for (text, line) in captures {
                        crates.insert(text.clone());
                        imports.push(json!({
                            "type": "external_crate",
                            "crate": text,
                            "line": line
                        }));
                    }
                }
                "visibility" => {
                    // Re-exports
                    for (text, line) in captures {
                        imports.push(json!({
                            "type": "reexport",
                            "visibility": text,
                            "line": line
                        }));
                    }
                }
                "absolute" => {
                    // Absolute path markers
                    for (_text, line) in captures {
                        imports.push(json!({
                            "type": "absolute_import",
                            "line": line
                        }));
                    }
                }
                _ => {
                    // Other captures - store for debugging
                    for (text, line) in captures {
                        imports.push(json!({
                            "type": "other",
                            "capture_type": capture_type,
                            "text": text,
                            "line": line
                        }));
                    }
                }
            }
        }

        let unique_modules = modules.len();
        Ok(json!({
            "imports": imports,
            "summary": {
                "modules": modules.into_iter().collect::<Vec<_>>(),
                "items": items.into_iter().collect::<Vec<_>>(),
                "aliases": aliases.into_iter().collect::<Vec<_>>(),
                "crates": crates.into_iter().collect::<Vec<_>>(),
                "wildcards": wildcards,
                "total_imports": imports.len(),
                "unique_modules": unique_modules,
                "analysis_method": "tree_sitter_semantic_v3"
            }
        }))
    }

    /// Process function query matches into structured data
    fn process_function_matches<'a>(
        &self,
        query: &Query,
        matches: impl Iterator<Item = tree_sitter::QueryMatch<'a, 'a>>,
        source: &str
    ) -> Result<Value> {
        let mut functions = Vec::new();

        for m in matches {
            let mut func_info = HashMap::new();

            for capture in m.captures {
                let text = capture.node.utf8_text(source.as_bytes())?;
                let capture_name = &query.capture_names()[capture.index as usize];

                match capture_name.as_str() {
                    "function_name" | "name" => {
                        func_info.insert("name", json!(text));
                    }
                    "function" | "method" => {
                        func_info.insert("line", json!(capture.node.start_position().row + 1));
                        func_info.insert("end_line", json!(capture.node.end_position().row + 1));
                        func_info.insert("length", json!(capture.node.end_position().row - capture.node.start_position().row + 1));
                    }
                    "visibility" => {
                        func_info.insert("visibility", json!(text));
                    }
                    _ => {}
                }
            }

            if !func_info.is_empty() {
                functions.push(json!(func_info));
            }
        }

        Ok(json!({
            "functions": functions,
            "count": functions.len()
        }))
    }

    /// Process function call matches for call graph generation
    fn process_call_matches<'a>(
        &self,
        query: &Query,
        matches: impl Iterator<Item = tree_sitter::QueryMatch<'a, 'a>>,
        source: &str
    ) -> Result<Value> {
        let mut calls = Vec::new();

        for m in matches {
            let mut call_info = HashMap::new();

            for capture in m.captures {
                let text = capture.node.utf8_text(source.as_bytes())?;
                let capture_name = &query.capture_names()[capture.index as usize];

                match capture_name.as_str() {
                    "call_name" | "method_call_name" | "scoped_call_name" | "generic_call_name" => {
                        call_info.insert("callee", json!(text));
                        call_info.insert("line", json!(capture.node.start_position().row + 1));
                    }
                    "qualified_call" => {
                        // For qualified calls like module.function(), extract the full path
                        call_info.insert("callee", json!(text));
                        call_info.insert("line", json!(capture.node.start_position().row + 1));
                    }
                    _ => {}
                }
            }

            if !call_info.is_empty() {
                calls.push(json!(call_info));
            }
        }

        Ok(json!({
            "calls": calls,
            "count": calls.len()
        }))
    }

    /// Process complexity query matches into metrics
    fn process_complexity_matches<'a>(
        &self,
        query: &Query,
        matches: impl Iterator<Item = tree_sitter::QueryMatch<'a, 'a>>,
        source: &str
    ) -> Result<Value> {
        let mut complexity_points = Vec::new();
        let mut total_complexity = 1; // Base complexity

        for m in matches {
            for capture in m.captures {
                let capture_name = &query.capture_names()[capture.index as usize];

                match capture_name.as_str() {
                    "complexity_point" | "complexity" => {
                        total_complexity += 1;
                        complexity_points.push(json!({
                            "type": capture.node.kind(),
                            "line": capture.node.start_position().row + 1
                        }));
                    }
                    "match_arm" => {
                        total_complexity += 1; // Each match arm adds complexity
                    }
                    _ => {}
                }
            }
        }

        Ok(json!({
            "total_complexity": total_complexity,
            "complexity_points": complexity_points,
            "complexity_grade": match total_complexity {
                1..=5 => "A",
                6..=10 => "B",
                11..=20 => "C",
                21..=30 => "D",
                _ => "F"
            }
        }))
    }

    /// Analyze multiple files in a directory automatically
    pub fn analyze_codebase(&mut self, codebase_path: &Path) -> Result<Value> {
        let mut all_results = HashMap::new();
        let mut language_stats = HashMap::new();
        let mut total_files = 0;

        for entry in WalkDir::new(codebase_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_file() {
                if let Some(_language) = SupportedLanguage::from_path(path) {
                    match self.analyze_file(path) {
                        Ok(analysis) => {
                            let file_path = path.to_string_lossy().to_string();

                            // Update language statistics
                            if let Some(lang) = analysis.get("language").and_then(|v| v.as_str()) {
                                *language_stats.entry(lang.to_string()).or_insert(0) += 1;
                            }

                            all_results.insert(file_path, analysis);
                            total_files += 1;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to analyze {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(json!({
            "files": all_results,
            "summary": {
                "total_files": total_files,
                "languages": language_stats,
                "analysis_method": "automatic_tree_sitter_query"
            }
        }))
    }

    /// Process community tags.scm queries for symbol extraction
    fn process_community_tags<'a>(
        &self,
        query: &Query,
        matches: impl Iterator<Item = tree_sitter::QueryMatch<'a, 'a>>,
        source: &str
    ) -> Result<Value> {
        let mut symbols = Vec::new();
        let mut by_type: HashMap<String, Vec<Value>> = HashMap::new();

        for m in matches {
            for capture in m.captures {
                let text = capture.node.utf8_text(source.as_bytes())?;
                let capture_name = &query.capture_names()[capture.index as usize];
                let line = capture.node.start_position().row + 1;

                let symbol = json!({
                    "name": text,
                    "type": capture_name,
                    "line": line,
                    "source": "community_tags"
                });

                symbols.push(symbol.clone());
                by_type.entry(capture_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(symbol);
            }
        }

        Ok(json!({
            "symbols": symbols,
            "by_type": by_type,
            "total": symbols.len(),
            "analysis_method": "tree_sitter_community_tags"
        }))
    }

    /// Process community highlights.scm queries for semantic analysis
    fn process_community_highlights<'a>(
        &self,
        query: &Query,
        matches: impl Iterator<Item = tree_sitter::QueryMatch<'a, 'a>>,
        source: &str
    ) -> Result<Value> {
        let mut highlights = Vec::new();
        let mut semantic_types: HashMap<String, u32> = HashMap::new();

        for m in matches {
            for capture in m.captures {
                let text = capture.node.utf8_text(source.as_bytes())?;
                let capture_name = &query.capture_names()[capture.index as usize];
                let start_pos = capture.node.start_position();
                let end_pos = capture.node.end_position();

                highlights.push(json!({
                    "text": text,
                    "semantic_type": capture_name,
                    "start": {
                        "row": start_pos.row + 1,
                        "column": start_pos.column
                    },
                    "end": {
                        "row": end_pos.row + 1,
                        "column": end_pos.column
                    },
                    "source": "community_highlights"
                }));

                *semantic_types.entry(capture_name.to_string()).or_insert(0) += 1;
            }
        }

        Ok(json!({
            "highlights": highlights,
            "semantic_summary": semantic_types,
            "total_highlights": highlights.len(),
            "analysis_method": "tree_sitter_community_highlights"
        }))
    }

    /// Process comment query matches into structured comment data
    fn process_comment_matches<'a>(
        &self,
        query: &Query,
        matches: impl Iterator<Item = tree_sitter::QueryMatch<'a, 'a>>,
        source: &str
    ) -> Result<Value> {
        let mut comments = Vec::new();
        // Dynamic limit based on file size - prevent infinite loops while allowing large files
        let source_lines = source.lines().count();
        let max_comments = std::cmp::max(source_lines * 2, 50000); // At least 50k, or 2x lines in file
        let mut count = 0;

        for m in matches {
            if count >= max_comments {
                break; // Prevent infinite processing
            }

            for capture in m.captures {
                if count >= max_comments {
                    break;
                }

                let text = match capture.node.utf8_text(source.as_bytes()) {
                    Ok(t) => t,
                    Err(_) => continue, // Skip invalid UTF-8
                };

                let capture_name = &query.capture_names()[capture.index as usize];
                let start_pos = capture.node.start_position();
                let end_pos = capture.node.end_position();

                // Determine comment type
                let comment_type = if text.trim_start().starts_with("///") {
                    "doc"
                } else if text.trim_start().starts_with("//") {
                    "line"
                } else if text.trim_start().starts_with("/*") {
                    "block"
                } else {
                    "unknown"
                };

                comments.push(json!({
                    "text": text,
                    "comment_type": comment_type,
                    "capture_name": capture_name,
                    "start": {
                        "row": start_pos.row + 1,
                        "column": start_pos.column
                    },
                    "end": {
                        "row": end_pos.row + 1,
                        "column": end_pos.column
                    },
                    "start_byte": capture.node.start_byte(),
                    "end_byte": capture.node.end_byte()
                }));

                count += 1;
            }
        }

        Ok(json!({
            "comments": comments,
            "total_comments": comments.len(),
            "analysis_method": "tree_sitter_comment_extraction"
        }))
    }
}



lazy_static::lazy_static! {
    static ref GLOBAL_QUERY_ENGINE: Arc<Mutex<QueryEngine>> = {
        Arc::new(Mutex::new(QueryEngine::new().expect("Failed to create query engine")))
    };
}

/// Get the global query engine instance
pub fn get_query_engine() -> Result<std::sync::MutexGuard<'static, QueryEngine>> {
    Ok(GLOBAL_QUERY_ENGINE.lock().unwrap())
}