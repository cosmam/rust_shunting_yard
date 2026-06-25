A repo for me to play around with implementations of the Shunting Yard Algorithm! And possibly related cases, depending on how long my attention holds.

## Background Assumptions

Any constants have already been replaced with their numerical values. So, for instance, neither pi nor π will be in the equation to evaluate; it will already have been replaced with 3.14159

Whitespace has no effect.

## Evaluation API

The compatibility entrypoint evaluates an expression with variables supplied by
a `HashMap<String, Value>`:

```rust
use shunting_yard::{evaluate, Value};
use std::collections::HashMap;

let mut variables = HashMap::new();
variables.insert("base".to_owned(), Value::Integer(4));

assert_eq!(
    evaluate("base + 2 * 3", &variables),
    Ok(Value::Integer(10))
);
```

For callback-based variable sources that are not a map, use `evaluate_with` and
provide a resolver callback that returns `Result<Value, EvalError>`:

```rust
use shunting_yard::{evaluate_with, EvalError, Value};

let result = evaluate_with("runtime_value + 2", |name| match name {
    "runtime_value" => Ok(Value::Integer(40)),
    other => Err(EvalError::UnknownVariable(other.to_owned())),
});

assert_eq!(result, Ok(Value::Integer(42)));
```

For a named resolver type, implement `VariableResolver` and pass it to
`evaluate_with_resolver`. Named resolvers are passed by value. The map-backed
APIs use a built-in borrowed `HashMap` adapter; other borrowed named resolvers
need their own explicit `VariableResolver` implementation for the borrowed type
or a small wrapper:

```rust
use shunting_yard::{evaluate_with_resolver, EvalError, Value, VariableResolver};

struct RuntimeResolver;

impl VariableResolver for RuntimeResolver {
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
        match name {
            "runtime_value" => Ok(Value::Integer(40)),
            other => Err(EvalError::UnknownVariable(other.to_owned())),
        }
    }
}

assert_eq!(
    evaluate_with_resolver("runtime_value + 2", RuntimeResolver),
    Ok(Value::Integer(42))
);
```

Use `evaluate_with_options` for map-backed evaluation with explicit limits, or
`evaluate_with_options_and_resolver` for named resolvers with explicit limits:

```rust
use shunting_yard::{
    evaluate_with_options_and_resolver, EvalError, EvalOptions, Value, VariableResolver,
};

struct RuntimeResolver;

impl VariableResolver for RuntimeResolver {
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
        match name {
            "runtime_value" => Ok(Value::Integer(40)),
            other => Err(EvalError::UnknownVariable(other.to_owned())),
        }
    }
}

let options = EvalOptions {
    max_tokens: 3,
    ..EvalOptions::default()
};

assert_eq!(
    evaluate_with_options_and_resolver("runtime_value + 2", RuntimeResolver, &options),
    Ok(Value::Integer(42))
);
```

All evaluation paths reject variable values and operation results that would
produce NaN, infinity, or subnormal floating-point values.

## Verification

The main local verification pass is:

```bash
cargo fmt --check
cargo test --all-targets --all-features
cargo test --release --all-targets --all-features
cargo test --doc --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

See [guidelines/verification_and_ci_plan.md](guidelines/verification_and_ci_plan.md)
for supply-chain checks, fuzzing, Miri, sanitizer commands, and CI expectations.

## Tokens to Parse

Broadly speaking, there are four categories of things to parse: numbers, operators, functions, and mistakes. Details are presented below

### Numbers

For maximum flexibility, I intend to support: integers, decimals, and scientific notation (e or E). Signs will be handled by unary operators

### Operators

Should allow for binary operators, with both left- and right-precedence if possible, and unary operators. A non-exhaustive list is:
* Binary operators: +, -, *, /, ^, %
* Unary operators: +, -
* Stretch goal binary operators: &, &&, |, ||, <<, >>
* Stretch goal unary operators: °, !, ~

### Functions

Should allow for binary and unary functions.
* Binary functions: min, max, pow, mod, rem, round
* Unary functions: cos, sin, tan, acos, asin, atan, abs, ln, log, floor, ceiling

### Mistakes

Anything not in the above.

## Super-Mega-Extra Stretch Goal

Support equality operators. Evaluate both sides of the equation, then the operator, and return True or False

Operators: ==, !=, /=, <=, >=, <, >, ~=

## Testing

Testing will be driven by files with a comma-separated string and number (except for the error cases), where a correct evaluation of the string yields the numerical value.

### Number Formats

Valid number format is: [A][.B][C][D]. At least one of A and B is required; If C or D is defined, the other must be.

* A: Any valid string of numbers. 1, 123, 000001, 100000
* B: A period followed by a valid string of numbers
* C: Either 'e' or 'E'
* D: Optionally, a sign, followed by digits (decimal not allowed). so "00001", "+12", "-00001", "-123"
