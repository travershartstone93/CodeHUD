; Rust function detection patterns

; Regular functions: fn foo() -> Result<()> { }
(function_item
  name: (identifier) @function_name) @function

; Associated functions in impl blocks
(impl_item
  body: (declaration_list
    (function_item
      name: (identifier) @method_name))) @method