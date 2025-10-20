;; Simple comment extraction for Rust
;; Basic queries that should work with any tree-sitter version

;; All line comments
(line_comment) @comment

;; All block comments
(block_comment) @comment