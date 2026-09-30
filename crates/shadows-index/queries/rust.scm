(const_item name: (identifier) @name) @definition.constant
(static_item name: (identifier) @name) @definition.constant
(function_signature_item name: (identifier) @name) @definition.method
(call_expression function: (scoped_identifier name: (identifier) @name)) @reference.call
(call_expression function: (generic_function function: (identifier) @name)) @reference.call
(call_expression function: (generic_function function: (scoped_identifier name: (identifier) @name))) @reference.call
(call_expression function: (generic_function function: (field_expression field: (field_identifier) @name))) @reference.call
(macro_invocation macro: (scoped_identifier name: (identifier) @name)) @reference.call
((type_identifier) @name @reference.type (#not-match? @name "^(_|Self)$"))
((scoped_identifier path: (identifier) @name) @reference.type (#match? @name "^[A-Z]"))
