; Python function call detection patterns

; Direct function calls: foo()
(call
  function: (identifier) @call_name) @call

; Method calls: obj.method()
(call
  function: (attribute
    attribute: (identifier) @method_call_name)) @method_call

; Module function calls: module.function()
(call
  function: (attribute) @qualified_call) @qualified_call_expr
