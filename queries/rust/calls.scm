; Rust function call detection patterns

; Direct function calls: foo()
(call_expression
  function: (identifier) @call_name) @call

; Method calls: obj.method()
(call_expression
  function: (field_expression
    field: (field_identifier) @method_call_name)) @method_call

; Qualified function calls: module::function()
(call_expression
  function: (scoped_identifier
    name: (identifier) @scoped_call_name)) @scoped_call

; Generic function calls: foo::<T>()
(call_expression
  function: (generic_function
    function: (identifier) @generic_call_name)) @generic_call
