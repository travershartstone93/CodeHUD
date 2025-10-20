; Rust imports - proper tree-sitter approach
; Leaf-centric semantic capture following community standards
; Addresses over-specific patterns and wildcard handling

; === SEMANTIC APPROACH ===
; Capture meaning, not syntax. Let tree-sitter handle structural complexity.

; Import declarations (top-level capture)
(use_declaration) @import

; External crate declarations
(extern_crate_declaration) @import

; === LEAF CAPTURES ===
; Individual imported items (the actual imports)
(use_declaration
  argument: (identifier) @item)

; Items from scoped paths
(scoped_identifier
  name: (identifier) @item)

; Items from use lists
(use_list
  (identifier) @item)

; === MODULE PATHS ===
; Module references of any kind
(identifier) @module
(crate) @module
(self) @module
(super) @module

; Path components
(scoped_identifier
  path: (_) @path)

(scoped_use_list
  path: (_) @path)

; === ALIASES ===
; Any alias, regardless of context
(use_as_clause
  alias: (identifier) @alias)

; === WILDCARDS ===
; Glob imports
(use_wildcard) @wildcard

; === VISIBILITY ===
; Re-exports and public imports
(visibility_modifier) @visibility

; === SPECIAL MARKERS ===
; Absolute path indicator
"::" @absolute

; Crate names
(extern_crate_declaration
  name: (identifier) @crate)