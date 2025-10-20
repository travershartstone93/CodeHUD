; Python import statement patterns
; For topology.rs Python analysis

; Basic imports: import os, sys
(import_statement
  name: (dotted_name) @module_name) @import

; From imports: from os import path
(import_from_statement
  module_name: (dotted_name) @module_name
  name: (dotted_name) @import_name) @from_import

; From imports with alias: from os import path as p
(import_from_statement
  module_name: (dotted_name) @module_name
  name: (aliased_import
    name: (dotted_name) @import_name
    alias: (identifier) @alias_name)) @aliased_from_import

; Import with alias: import numpy as np
(import_statement
  name: (aliased_import
    name: (dotted_name) @module_name
    alias: (identifier) @alias_name)) @aliased_import

; Grouped from imports: from os import path, environ
(import_from_statement
  module_name: (dotted_name) @module_name
  name: (dotted_name) @import_name) @grouped_import

; Wildcard imports: from os import *
; TODO: Fix wildcard import pattern - tree-sitter says it's "impossible"
; (import_from_statement
;   module_name: (dotted_name) @module_name
;   name: (wildcard_import)) @wildcard_import

; Relative imports: from .module import function
(import_from_statement
  module_name: (relative_import) @relative_module
  name: (dotted_name) @import_name) @relative_import

; Future imports: from __future__ import annotations
(import_from_statement
  module_name: (dotted_name) @future_module
  name: (dotted_name) @future_import
  (#match? @future_module "__future__")) @future_import