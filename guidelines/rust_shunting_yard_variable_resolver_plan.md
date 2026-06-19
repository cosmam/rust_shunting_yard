# Public Variable Resolver Interface Plan for `rust_shunting_yard`

## Context

On the `feature/proptest` branch, the current public interface evaluates an expression by passing a `HashMap<String, Value>` into `evaluate`:

```rust
pub fn evaluate(text: &str, variables: &HashMap<String, Value>) -> Result<Value, EvalError>
```

Internally, parsing and evaluation flow through:

```text
evaluate
  -> evaluate_tokens
  -> eval::eval
  -> Expression::Variable(name)
  -> variables.get(name)
```

The desired change is to preserve the existing `HashMap`-based API while also allowing callers to provide a callback that receives a variable name and returns a `Value`, with errors allowed. This is also intended to support a future FFI boundary using a Linux-kernel-style approach: isolate raw FFI at the boundary, expose safe Rust abstractions internally, and avoid spreading unsafe code through the evaluator.

The primary design goal is:

> All modules below `lib.rs` should use one implementation path, not separate evaluator paths for maps and callbacks.

## Recommendation

Make variable lookup a single resolver abstraction. Keep the existing `HashMap` API as a thin compatibility and ergonomics wrapper over that abstraction.

The core evaluator should not know whether variables come from:

- a `HashMap`,
- a closure,
- a cache,
- a runtime object,
- a C/FFI callback,
- or some future resolver implementation.

It should only know how to ask for a variable by name.

## Proposed Public Resolver Trait

Add a public resolver trait near `Value` and `EvalError` in `lib.rs`:

```rust
pub trait VariableResolver {
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError>;
}
```

Then implement it for closures:

```rust
impl<F> VariableResolver for F
where
    F: FnMut(&str) -> Result<Value, EvalError>,
{
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
        self(name)
    }
}
```

This lets users pass closures directly:

```rust
let result = evaluate_with("runtime_value + 2", |name| {
    match name {
        "runtime_value" => Ok(Value::Integer(40)),
        other => Err(EvalError::UnknownVariable(other.to_owned())),
    }
});

assert_eq!(result, Ok(Value::Integer(42)));
```

## Preserve the Existing `HashMap` API

Keep the existing public `evaluate` signature:

```rust
pub fn evaluate(text: &str, variables: &HashMap<String, Value>) -> Result<Value, EvalError> {
    evaluate_with(text, |name| {
        variables
            .get(name)
            .cloned()
            .ok_or_else(|| EvalError::UnknownVariable(name.to_owned()))
    })
}
```

This preserves current callers and tests:

```rust
let mut variables = HashMap::new();
variables.insert("base".to_owned(), Value::Integer(4));

assert_eq!(
    evaluate("base + 2", &variables),
    Ok(Value::Integer(6))
);
```

## Add the New Callback-Based API

Add a new public function:

```rust
pub fn evaluate_with<R>(text: &str, resolver: R) -> Result<Value, EvalError>
where
    R: VariableResolver,
{
    let lexer = lexer::Lexer::new(text);
    evaluate_tokens_with(lexer, resolver)
}
```

Then make the token-based internal path use the same resolver:

```rust
fn evaluate_tokens_with<'input, Tokens, R>(
    tokens: Tokens,
    mut resolver: R,
) -> Result<Value, EvalError>
where
    Tokens: IntoIterator<Item = lexer::Spanned<tokens::Token<'input>, usize, tokens::LexicalError>>,
    R: VariableResolver,
{
    let parser = calc::ExpressionParser::new();

    let mut errors = Vec::new();
    let result = parser.parse(&mut errors, tokens);

    match result {
        Ok(ast) => eval::eval(&ast, &mut resolver),
        Err(_) => Err(EvalError::ParserError),
    }
}
```

The old `evaluate_tokens` can either be renamed to `evaluate_tokens_with` or kept as a small test-only adapter. The important point is that the canonical implementation path should be resolver-based.

## Update `eval.rs`

Change `eval::eval` from a `HashMap`-specific function:

```rust
pub fn eval(expr: &Expression, variables: &HashMap<String, Value>) -> Result<Value, EvalError>
```

to a resolver-based function:

```rust
pub fn eval<R>(expr: &Expression, resolver: &mut R) -> Result<Value, EvalError>
where
    R: VariableResolver + ?Sized,
{
    match expr {
        Expression::Bool(n) => Ok(Value::Bool(*n)),
        Expression::Integer(n) => Ok(Value::Integer(*n)),
        Expression::Float(n) => Ok(Value::Float(*n)),

        Expression::UnaryOperation { operator, value } => {
            let value = eval(value, resolver)?;
            apply_unary(operator, value)
        }

        Expression::BinaryOperation { lhs, operator, rhs } => {
            let left = eval(lhs, resolver)?;
            let right = eval(rhs, resolver)?;
            apply_binary(operator, left, right)
        }

        Expression::Function { func, arguments } => {
            let values = arguments
                .iter()
                .map(|v| eval(v, resolver))
                .collect::<Result<Vec<_>, _>>()?;

            apply_function(func, values)
        }

        Expression::Variable(name) => resolver.resolve(name),

        Expression::Error | Expression::LexicalError(_) => Err(EvalError::InvalidExpression),
    }
}
```

Only the variable lookup logic changes. The operator and function evaluation logic stays untouched.

## Why This Shape Is Preferable

### Use `&str`, Not `String`

The resolver should receive `&str`:

```rust
fn resolve(&mut self, name: &str) -> Result<Value, EvalError>;
```

This avoids allocating for every variable lookup. The resolver can allocate only when needed, such as when constructing an error.

### Use `FnMut`, Not `Fn`

The callback implementation should accept `FnMut`:

```rust
F: FnMut(&str) -> Result<Value, EvalError>
```

This allows future resolvers to mutate captured state. Examples include:

- caching resolved variables,
- recording diagnostics,
- counting lookups in tests,
- managing FFI-owned context,
- reusing temporary buffers,
- or forwarding through a mutable runtime handle.

`Fn` would be too restrictive for these cases.

### Return Owned `Value`

The resolver should return an owned `Value`:

```rust
Result<Value, EvalError>
```

This avoids lifetime coupling between the evaluator and the storage behind the resolver.

The current `HashMap` behavior already clones values before returning them, so this preserves the current semantic model.

### Return `EvalError`, Not `Option<Value>`

The callback should return:

```rust
Result<Value, EvalError>
```

not:

```rust
Option<Value>
```

The goal is not just “found” versus “missing.” Callback-based resolution may fail for many reasons:

- unknown variable,
- backend lookup failure,
- invalid runtime value,
- FFI conversion failure,
- unavailable external context,
- or other resolver-specific errors.

The crate already has a public error type, so the callback should participate in that error model.

## Error Design

Avoid returning `Box<dyn Error>` from the resolver for now.

`EvalError` currently derives traits useful for tests and property tests, such as `Clone` and `PartialEq`. A boxed dynamic source error would make those derives awkward or impossible.

For now, have callbacks return `EvalError` directly:

```rust
FnMut(&str) -> Result<Value, EvalError>
```

If a distinct resolver error is needed later, add a concrete enum variant:

```rust
#[error("variable lookup failed for {name}: {message}")]
VariableLookupFailed {
    name: String,
    message: String,
}
```

For FFI-specific failures, use a concrete and testable representation:

```rust
#[error("variable lookup failed for {name} with status {status}: {message}")]
VariableLookupFailed {
    name: String,
    status: i32,
    message: String,
}
```

This keeps the error type comparable, testable, and compatible with property testing.

## FFI Direction

Do not let FFI drive the evaluator design.

The Rust evaluator should expose a safe abstraction:

```text
VariableResolver
```

The eventual FFI layer should adapt an external C-style callback into that abstraction:

```text
C ABI callback
    -> unsafe extern "C" trampoline
    -> safe FfiResolver
    -> VariableResolver trait
    -> eval::eval
```

Avoid this shape:

```text
eval::eval
    -> raw extern "C" callback
```

`eval.rs` should never know that FFI exists.

A future FFI adapter could look conceptually like:

```rust
pub(crate) struct FfiResolver {
    callback: ffi::LookupCallback,
    user_data: *mut core::ffi::c_void,
}

impl VariableResolver for FfiResolver {
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
        // Convert `name` to the ABI representation.
        // Call the raw callback.
        // Convert status/out-value back into `Result<Value, EvalError>`.
        // Keep all unsafe and ABI rules here.
    }
}
```

The important boundary rule is:

> Unsafe FFI code should live in a small, reviewed adapter module. The evaluator should consume only safe Rust abstractions.

When implementing the FFI layer later, prefer:

- `#[repr(C)]` or `#[repr(transparent)]` ABI structs,
- explicit status codes,
- out-parameters for returned values,
- checked conversions into `Value`,
- documented `# Safety` sections on unsafe APIs,
- `// SAFETY:` comments for unsafe blocks,
- no panics across the FFI boundary,
- no raw pointers exposed to the evaluator,
- and no direct raw C callback invocation from recursive evaluation logic.

## Incremental Implementation Plan

### Step 1: Add the Resolver Trait

Add `VariableResolver` to `lib.rs` near `Value` and `EvalError`.

```rust
pub trait VariableResolver {
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError>;
}
```

Add the blanket implementation for closures:

```rust
impl<F> VariableResolver for F
where
    F: FnMut(&str) -> Result<Value, EvalError>,
{
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
        self(name)
    }
}
```

### Step 2: Add `evaluate_with`

Add the new callback-oriented public API:

```rust
pub fn evaluate_with<R>(text: &str, resolver: R) -> Result<Value, EvalError>
where
    R: VariableResolver,
{
    let lexer = lexer::Lexer::new(text);
    evaluate_tokens_with(lexer, resolver)
}
```

### Step 3: Rewrite `evaluate` as an Adapter

Keep the existing public interface but make it delegate:

```rust
pub fn evaluate(text: &str, variables: &HashMap<String, Value>) -> Result<Value, EvalError> {
    evaluate_with(text, |name| {
        variables
            .get(name)
            .cloned()
            .ok_or_else(|| EvalError::UnknownVariable(name.to_owned()))
    })
}
```

### Step 4: Convert `evaluate_tokens`

Change the internal token evaluator to accept a resolver.

```rust
fn evaluate_tokens_with<'input, Tokens, R>(
    tokens: Tokens,
    mut resolver: R,
) -> Result<Value, EvalError>
where
    Tokens: IntoIterator<Item = lexer::Spanned<tokens::Token<'input>, usize, tokens::LexicalError>>,
    R: VariableResolver,
{
    let parser = calc::ExpressionParser::new();

    let mut errors = Vec::new();
    let result = parser.parse(&mut errors, tokens);

    match result {
        Ok(ast) => eval::eval(&ast, &mut resolver),
        Err(_) => Err(EvalError::ParserError),
    }
}
```

### Step 5: Convert `eval::eval`

Change `eval::eval` to accept a resolver rather than a `HashMap`.

```rust
pub fn eval<R>(expr: &Expression, resolver: &mut R) -> Result<Value, EvalError>
where
    R: VariableResolver + ?Sized,
```

Update recursive calls to pass `resolver`.

Change only the variable arm:

```rust
Expression::Variable(name) => resolver.resolve(name),
```

### Step 6: Update Existing Tests

Existing tests that use `evaluate(..., &HashMap::new())` should continue to pass once `evaluate` delegates through the resolver.

Any direct tests of `evaluate_tokens` should either:

- switch to `evaluate_tokens_with(tokens, |_| Err(...))`, or
- use a small compatibility adapter for token tests.

### Step 7: Add Callback Tests

Add tests for:

1. callback resolves a known variable;
2. callback returns `UnknownVariable`;
3. callback can return a resolver-specific error;
4. callback can mutate state, proving `FnMut`;
5. `HashMap` API and callback API return equivalent results.

Example:

```rust
#[test]
fn evaluate_with_resolves_variable_from_callback() {
    let result = evaluate_with("runtime_value + 2", |name| {
        match name {
            "runtime_value" => Ok(Value::Integer(40)),
            other => Err(EvalError::UnknownVariable(other.to_owned())),
        }
    });

    assert_eq!(result, Ok(Value::Integer(42)));
}
```

Example showing `FnMut`:

```rust
#[test]
fn evaluate_with_allows_mutating_resolver_state() {
    let mut lookups = 0;

    let result = evaluate_with("x + x", |name| {
        lookups += 1;

        match name {
            "x" => Ok(Value::Integer(2)),
            other => Err(EvalError::UnknownVariable(other.to_owned())),
        }
    });

    assert_eq!(result, Ok(Value::Integer(4)));
    assert_eq!(lookups, 2);
}
```

### Step 8: Add Property Tests

Since the branch already uses `proptest`, add a property that evaluates the same generated expression through both APIs.

For example:

```rust
proptest! {
    #[test]
    fn prop_hashmap_and_callback_resolution_match(
        name in variable_name(),
        value in -1_000_000i64..1_000_000,
    ) {
        let mut variables = HashMap::new();
        variables.insert(name.clone(), Value::Integer(value));

        let via_map = evaluate(&name, &variables);
        let via_callback = evaluate_with(&name, |lookup| {
            variables
                .get(lookup)
                .cloned()
                .ok_or_else(|| EvalError::UnknownVariable(lookup.to_owned()))
        });

        prop_assert_eq!(via_map, via_callback);
    }
}
```

This verifies that the old public API is truly just an adapter over the new resolver path.

## Suggested Final Public API

The public API should expose both:

```rust
pub fn evaluate(text: &str, variables: &HashMap<String, Value>) -> Result<Value, EvalError>;

pub fn evaluate_with<R>(text: &str, resolver: R) -> Result<Value, EvalError>
where
    R: VariableResolver;
```

And the public trait:

```rust
pub trait VariableResolver {
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError>;
}
```

## Suggested Module Boundary

Recommended structure:

```text
src/
  lib.rs
    - public Value
    - public EvalError
    - public VariableResolver
    - public evaluate
    - public evaluate_with
    - parser/token orchestration

  eval.rs
    - resolver-based AST evaluation
    - no HashMap dependency
    - no FFI dependency

  ffi.rs, later
    - raw ABI types
    - unsafe callback trampoline
    - FfiResolver
    - conversion into safe Value/EvalError
```

After this change, `eval.rs` should not need:

```rust
use std::collections::HashMap;
```

That dependency should remain only in `lib.rs`, where the compatibility wrapper lives.

## Bottom Line

Use a `VariableResolver` trait as the single internal variable-lookup abstraction.

Keep:

```rust
evaluate(text, &variables)
```

as the existing `HashMap` API.

Add:

```rust
evaluate_with(text, resolver)
```

for callbacks.

Make both flow into the same resolver-based evaluator.

Later, implement FFI as a small adapter that implements `VariableResolver`, keeping raw callbacks and unsafe code out of the evaluator itself.
