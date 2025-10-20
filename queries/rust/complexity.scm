; Rust complexity analysis patterns
; Each match contributes to cyclomatic complexity

; Conditional expressions
(if_expression) @complexity_point
(match_expression) @complexity_point

; Loop constructs
(while_expression) @complexity_point
(for_expression) @complexity_point
(loop_expression) @complexity_point

; Match arms (each arm adds complexity)
(match_expression
  body: (match_block
    (match_arm) @match_arm))

; Result/Option handling
(try_expression) @complexity_point

; Closure expressions
(closure_expression) @complexity_point

; Early returns that add branching
(return_expression) @early_return

; Panic/unreachable branches
(macro_invocation
  macro: (identifier) @macro_name
  (#match? @macro_name "^(panic|unreachable|todo|unimplemented)$")) @panic_branch

; Nested functions (increase complexity)
(function_item
  body: (block
    (function_item) @nested_function))

; Error propagation with ?
(try_expression
  (field_expression) @error_propagation)

; Conditional compilation
(attribute_item
  (attribute
    (identifier) @attr_name
    (#match? @attr_name "^cfg$"))) @conditional_compilation