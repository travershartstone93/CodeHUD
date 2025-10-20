; Rust import analysis - leaf-centric approach
; Following community standards and your feedback about over-specific patterns
; Focus on capturing leaf imports and letting tree-sitter handle the complexity

; === Core Strategy: Capture every leaf import ===
; Rather than trying to enumerate every top-level use shape,
; capture every leaf import (a single item possibly with alias)

; Leaf imports: any single imported item
(identifier) @import.item
(
  (identifier) @import.item
  .
  (identifier)? @import.alias
)

; Module paths: capture the full path leading to imports
(scoped_identifier
  path: (_) @import.module_path
  name: (identifier) @import.item)

; Special module references
(crate) @import.module_path
(self) @import.module_path
(super) @import.module_path

; Wildcard imports - the * itself
"*" @import.wildcard

; Use declarations - broad capture
(use_declaration) @import.declaration

; External crate declarations
(extern_crate_declaration
  name: (identifier) @import.crate
  alias: (identifier)? @import.alias)

; === Visibility patterns ===
(visibility_modifier) @import.visibility

; === Alias patterns ===
; Capture aliases anywhere they appear
(use_as_clause
  path: (_) @import.source
  alias: (identifier) @import.alias)

; === Group imports ===
; Let tree-sitter handle nested structures, just capture the lists
(use_list) @import.group
(scoped_use_list) @import.scoped_group

; === Complex path handling ===
; Leading :: (absolute paths)
("::" @import.absolute_marker)

; === Re-exports ===
; pub use declarations
(use_declaration
  visibility: (visibility_modifier) @import.reexport_visibility)

; === Macro imports ===
(macro_invocation
  macro: (identifier) @import.macro_name
  "!" @import.macro_marker)