; Python function detection patterns

; Regular functions: def foo(): ...
(function_definition
  name: (identifier) @function_name
  body: (block) @function_body) @function

; Async functions: async def foo(): ...
(function_definition
  "async"
  name: (identifier) @async_function_name
  body: (block) @function_body) @async_function

; Methods in classes: class Foo: def bar(self): ...
(class_definition
  body: (block
    (function_definition
      name: (identifier) @method_name
      body: (block) @method_body))) @method

; Lambda functions: lambda x: x + 1
(lambda
  parameters: (lambda_parameters) @lambda_params
  body: (expression) @lambda_body) @lambda_function