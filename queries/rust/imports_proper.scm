; Rust import analysis following tree-sitter best practices
; Leaf-centric approach - capture semantic meaning, not structure
; Based on community standards and addressing over-specific patterns

; === PRIMARY STRATEGY ===
; Capture semantic elements rather than trying to match every syntactic shape
; Let tree-sitter's parser handle the complexity of nested structures

; === Import declarations ===
; Capture the whole declaration for context
(use_declaration) @import

; === Modules and crates ===
; Capture any module reference
(identifier) @module (#match? @module "^[a-z][a-z0-9_]*$")
(crate) @module
(self) @module
(super) @module

; Capture crate declarations
(extern_crate_declaration
  name: (identifier) @crate)

; === Imported items ===
; Any identifier that's being imported (leaf approach)
(use_declaration
  argument: (identifier) @item)

; Items in scoped imports
(scoped_identifier
  name: (identifier) @item)

; Items in use lists
(use_list
  (identifier) @item)

; === Aliases ===
; Capture any alias, regardless of context
(use_as_clause
  alias: (identifier) @alias)

; === Paths ===
; Capture path components without over-specifying structure
(scoped_identifier
  path: (_) @path)

(scoped_use_list
  path: (_) @path)

; === Wildcards ===
; Simple wildcard capture
(use_wildcard) @wildcard

; === Visibility ===
; Re-exports and visibility
(visibility_modifier) @visibility

; === Special patterns ===
; Absolute paths (leading ::)
"::" @absolute

; Macro imports
(macro_invocation
  macro: (identifier) @macro)