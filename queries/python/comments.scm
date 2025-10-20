;; Simple Python comments query - just extract all comments and docstrings
(comment) @comment

;; Function docstrings
(function_definition
  body: (block
    (expression_statement
      (string) @comment) .) )

;; Class docstrings
(class_definition
  body: (block
    (expression_statement
      (string) @comment) .) )

;; Module docstrings
(module
  (expression_statement
    (string) @comment) .)