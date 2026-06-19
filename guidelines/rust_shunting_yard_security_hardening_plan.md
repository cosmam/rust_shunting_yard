# Rust Shunting Yard High-Security Review and Hardening Plan

This document is an implementation-oriented security review of the Rust `shunting_yard` crate. It is written to be co-located with the repository and used as a hardening checklist.

## Scope

Reviewed target:

- Repository: `cosmam/shunting-yards`
- Crate path: `rust/shunting_yard`
- Reviewed branch/ref: current accessible `main`
- Requested branch: `rust_shunting_yard`

I could not find a literal `rust_shunting_yard` branch ref during review, so this document is based on the current accessible Rust implementation under `rust/shunting_yard`.

## Security posture summary

The crate has a reasonable high-level architecture:

- Public API returns `Result<Value, EvalError>`.
- Lexer, parser, AST, and evaluator are separated.
- Lexical errors are represented and can flow into the AST.
- The evaluator has explicit error variants for invalid expressions, invalid types, math errors, parser errors, and unknown variables.
- There are many example-driven parser and evaluator tests.

However, it is **not high-security-ready** yet.

Under a hostile-input model, the evaluator currently has externally triggerable crash paths, release/debug behavior differences, weak numeric invariants, unbounded resource usage, and coarse error reporting. The main risk is not currently direct memory unsafety from hand-written `unsafe`; the reviewed code appears to be safe Rust. The main risk is that invalid or malicious input can produce panics, non-finite values, semantic corruption, or denial of service.

For this project, treat all of the following as security bugs:

- Panics from public APIs.
- Debug/release arithmetic divergence.
- Silent integer overflow.
- Non-finite floating-point results escaping evaluation.
- Invalid shift counts.
- Division, modulo, or Euclidean remainder by zero.
- Unbounded recursion or unbounded allocation from input.
- Collapsed error categories that make failures untestable.

## Severity summary

| Severity | Finding |
|---|---|
| Critical | Unchecked integer arithmetic in evaluator |
| Critical | Unvalidated bitshift counts |
| Critical | Division/modulo/remainder panic cases |
| Critical | Parser/evaluator resource exhaustion |
| High | Float lexer rejects NaN/Inf, but evaluator can create NaN/Inf |
| High | Mixed int/float conversion silently loses precision |
| High | Parser recovery diagnostics are discarded |
| Medium | CLI panics on ordinary I/O errors |
| Medium | `build.rs` uses `unwrap` |
| Medium | Error taxonomy relies on strings |
| Medium | Approximate equality is mathematically wrong for negatives and near zero |
| Medium | Float-to-int conversions are boundary-sensitive and insufficiently guarded |
| Medium | Unsafe policy is not enforced at crate/lint level |
| Medium | Tests are example-heavy but not hostile-input-driven |

---

# Implementation plan

## Phase 1: Stop public API panics

Goal: `evaluate(...)` must not panic for any UTF-8 input or any valid caller-provided variable map.

### 1.1 Replace unchecked integer arithmetic

Current risk examples include:

- Unary negation of `i64::MIN`.
- Addition overflow.
- Subtraction overflow.
- Multiplication overflow.
- Division overflow: `i64::MIN / -1`.
- Modulo overflow: `i64::MIN % -1`.
- Function-style modulo/remainder by zero.

Create helper functions and route all integer arithmetic through them.

```rust
fn checked_add_i64(lhs: i64, rhs: i64) -> Result<i64, EvalError> {
    lhs.checked_add(rhs)
        .ok_or(EvalError::IntegerOverflow { op: ArithmeticOp::Add })
}

fn checked_sub_i64(lhs: i64, rhs: i64) -> Result<i64, EvalError> {
    lhs.checked_sub(rhs)
        .ok_or(EvalError::IntegerOverflow { op: ArithmeticOp::Subtract })
}

fn checked_mul_i64(lhs: i64, rhs: i64) -> Result<i64, EvalError> {
    lhs.checked_mul(rhs)
        .ok_or(EvalError::IntegerOverflow { op: ArithmeticOp::Multiply })
}

fn checked_neg_i64(value: i64) -> Result<i64, EvalError> {
    value.checked_neg()
        .ok_or(EvalError::IntegerOverflow { op: ArithmeticOp::Negate })
}

fn checked_div_i64(lhs: i64, rhs: i64) -> Result<i64, EvalError> {
    if rhs == 0 {
        return Err(EvalError::DivisionByZero);
    }

    lhs.checked_div(rhs)
        .ok_or(EvalError::IntegerOverflow { op: ArithmeticOp::Divide })
}

fn checked_rem_i64(lhs: i64, rhs: i64) -> Result<i64, EvalError> {
    if rhs == 0 {
        return Err(EvalError::DivisionByZero);
    }

    lhs.checked_rem(rhs)
        .ok_or(EvalError::IntegerOverflow { op: ArithmeticOp::Modulo })
}

fn checked_rem_euclid_i64(lhs: i64, rhs: i64) -> Result<i64, EvalError> {
    if rhs == 0 {
        return Err(EvalError::DivisionByZero);
    }

    lhs.checked_rem_euclid(rhs)
        .ok_or(EvalError::IntegerOverflow { op: ArithmeticOp::Remainder })
}
```

Suggested structured error support:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Remainder,
    Power,
    Negate,
    ShiftLeft,
    ShiftRight,
    FloatToInteger,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvalError {
    InvalidArity,
    InvalidExpression,
    InvalidType {
        expected: &'static str,
        actual: &'static str,
    },
    DivisionByZero,
    IntegerOverflow {
        op: ArithmeticOp,
    },
    InvalidShiftCount {
        count: i64,
    },
    NonFiniteFloat,
    PrecisionLoss,
    ParserError,
    UnexpectedOpcode,
    UnknownVariable(String),
}
```

Do not keep math errors as arbitrary strings long-term. Strings are hard to match, hard to test, and easy to accidentally change.

### 1.2 Validate shift counts

Never shift by a signed user-controlled value directly.

```rust
fn checked_shift_count(count: i64) -> Result<u32, EvalError> {
    let count_u32 = u32::try_from(count)
        .map_err(|_| EvalError::InvalidShiftCount { count })?;

    if count_u32 >= i64::BITS {
        return Err(EvalError::InvalidShiftCount { count });
    }

    Ok(count_u32)
}

fn checked_shl_i64(lhs: i64, rhs: i64) -> Result<i64, EvalError> {
    let rhs = checked_shift_count(rhs)?;
    lhs.checked_shl(rhs)
        .ok_or(EvalError::InvalidShiftCount {
            count: i64::from(rhs),
        })
}

fn checked_shr_i64(lhs: i64, rhs: i64) -> Result<i64, EvalError> {
    let rhs = checked_shift_count(rhs)?;
    lhs.checked_shr(rhs)
        .ok_or(EvalError::InvalidShiftCount {
            count: i64::from(rhs),
        })
}
```

### 1.3 Add panic-free regression tests

Add a test module focused only on hostile inputs. These tests should assert that the public API returns `Result` and never unwinds.

```rust
#[test]
fn hostile_inputs_do_not_panic() {
    use std::collections::HashMap;

    let variables = HashMap::new();

    let cases = [
        "9223372036854775807 + 1",
        "9223372036854775807 * 2",
        "-9223372036854775808 - 1",
        "mod(1, 0)",
        "rem(1, 0)",
        "1 % 0",
        "1 / 0",
        "1 << -1",
        "1 << 64",
        "1 >> -1",
        "1 >> 64",
        "ln(-1)",
        "acos(2)",
        "asin(2)",
        "exp(10000)",
        "1e308 * 1e308",
        "((((((((((((((((((((1))))))))))))))))))))",
    ];

    for case in cases {
        let result = std::panic::catch_unwind(|| {
            let _ = shunting_yard::evaluate(case, &variables);
        });

        assert!(result.is_ok(), "evaluate panicked for input: {case:?}");
    }
}
```

Also test variable-provided boundary values, because some values cannot be represented directly in source syntax if unary minus is parsed as an operator.

```rust
#[test]
fn hostile_variable_values_do_not_panic() {
    use std::collections::HashMap;

    let mut variables = HashMap::new();
    variables.insert("min".to_string(), shunting_yard::Value::Integer(i64::MIN));
    variables.insert("max".to_string(), shunting_yard::Value::Integer(i64::MAX));

    let cases = [
        "-min",
        "min / -1",
        "min % -1",
        "rem(min, -1)",
        "max + 1",
        "max * 2",
    ];

    for case in cases {
        let result = std::panic::catch_unwind(|| {
            let _ = shunting_yard::evaluate(case, &variables);
        });

        assert!(result.is_ok(), "evaluate panicked for input: {case:?}");
    }
}
```

### 1.4 Enable release overflow checks

Add this while hardening:

```toml
[profile.release]
overflow-checks = true
```

This is not a substitute for checked arithmetic. It prevents silent release-mode corruption while the evaluator is still being hardened.

### 1.5 Remove `expect` and `unwrap` from non-test code

`main.rs` should not panic on ordinary I/O failure.

```rust
io::stdout().flush()?;
io::stdin().read_line(&mut input)?;
```

`build.rs` should return a proper error.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    lalrpop::process_root()?;
    Ok(())
}
```

---

## Phase 2: Enforce numeric invariants

Goal: If the lexer rejects NaN and infinity, the evaluator must not produce them either.

### 2.1 Introduce a finite float wrapper

Current code validates float literals, but operations like `ln(-1)`, `acos(2)`, `exp(10000)`, and `pow(1e308, 2)` can still produce `NaN` or infinity.

Create a refined type:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    pub fn new(value: f64) -> Result<Self, EvalError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(EvalError::NonFiniteFloat)
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}
```

Then change:

```rust
pub enum Value {
    Bool(bool),
    Integer(i64),
    Float(f64),
}
```

to:

```rust
pub enum Value {
    Bool(bool),
    Integer(i64),
    Float(FiniteF64),
}
```

This forces every float-producing path through validation.

### 2.2 Validate all float operation outputs

Wrap every float result:

```rust
fn checked_float(value: f64) -> Result<Value, EvalError> {
    FiniteF64::new(value).map(Value::Float)
}
```

Examples:

```rust
(Opcode::Multiply, Value::Float(l), Value::Float(r)) => {
    checked_float(l.get() * r.get())
}

(Opcode::Power, Value::Float(l), Value::Float(r)) => {
    checked_float(l.get().powf(r.get()))
}

Func::Ln => apply_float_unary(value, |v| v.ln())
```

where:

```rust
fn apply_float_unary(val: Value, op: fn(f64) -> f64) -> Result<Value, EvalError> {
    match val {
        Value::Float(value) => checked_float(op(value.get())),
        Value::Bool(_) | Value::Integer(_) => Err(EvalError::UnexpectedOpcode),
    }
}
```

### 2.3 Decide mixed integer/float semantics

Current conversions use `i64 as f64`, which silently loses precision for large integers.

Pick and document one policy:

#### Option A: Strict precision policy

Reject integer-to-float promotion unless the integer can be represented exactly.

```rust
fn i64_to_exact_f64(value: i64) -> Result<f64, EvalError> {
    const MAX_EXACT: i64 = 9_007_199_254_740_992; // 2^53

    if (-MAX_EXACT..=MAX_EXACT).contains(&value) {
        Ok(value as f64)
    } else {
        Err(EvalError::PrecisionLoss)
    }
}
```

#### Option B: Calculator-style promotion

Allow lossy promotion, but document it explicitly and test it.

For high-security behavior, prefer Option A unless this crate is intentionally a loose calculator.

### 2.4 Fix approximate equality

Current approximate equality should be replaced with an absolute/relative comparison.

```rust
const EPSILON: f64 = 0.000001;

fn approximately_equal(lhs: f64, rhs: f64) -> bool {
    let scale = lhs.abs().max(rhs.abs()).max(1.0);
    (lhs - rhs).abs() <= EPSILON * scale
}
```

Test cases:

```rust
#[test]
fn approximate_equality_handles_negative_values() {
    assert!(approximately_equal(-1.0000001, -1.0000002));
}

#[test]
fn approximate_equality_handles_near_zero_values() {
    assert!(approximately_equal(0.0, 0.0000001));
}
```

### 2.5 Harden float-to-int conversions

Avoid relying on loose `as i64` behavior after a simple `i64::MAX as f64` comparison. Centralize conversion.

```rust
fn f64_to_i64_checked(value: f64) -> Result<i64, EvalError> {
    if !value.is_finite() {
        return Err(EvalError::NonFiniteFloat);
    }

    if value < i64::MIN as f64 || value >= 9_223_372_036_854_775_808.0 {
        return Err(EvalError::IntegerOverflow {
            op: ArithmeticOp::FloatToInteger,
        });
    }

    Ok(value as i64)
}
```

Add boundary tests for:

- `i64::MIN as f64`
- `i64::MAX as f64`
- `2^63`
- `-2^63`
- adjacent representable values around those thresholds

---

## Phase 3: Improve parse/eval API and error taxonomy

Goal: Make every failure testable, diagnosable, and stable.

### 3.1 Split parsing and evaluation

Add separate stages:

```rust
pub fn parse(text: &str) -> Result<Expression<'_>, ParseErrors>;

pub fn evaluate_ast(
    expr: &Expression<'_>,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError>;

pub fn evaluate(
    text: &str,
    variables: &HashMap<String, Value>,
) -> Result<Value, Error>;
```

Suggested top-level error type:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Lexical(LexicalDiagnostic),
    Parse(ParseDiagnostics),
    Eval(EvalError),
    ResourceLimit(ResourceLimitError),
}
```

### 3.2 Preserve parser recovery diagnostics

The parser already receives an `errors` vector. Do not discard it.

Return diagnostics with:

- byte span
- found token
- expected tokens, if available
- lexical error kind
- recovery behavior, if relevant

Example shape:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct ParseDiagnostic {
    pub span: std::ops::Range<usize>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParseDiagnostics {
    pub diagnostics: Vec<ParseDiagnostic>,
}
```

### 3.3 Replace stringly typed errors

Avoid:

```rust
InvalidType(String)
MathError(String)
```

Prefer structured variants:

```rust
pub enum EvalError {
    InvalidArity {
        expected: Arity,
        actual: usize,
    },
    InvalidType {
        operation: OperationKind,
        expected: &'static str,
        actual: &'static str,
    },
    DivisionByZero,
    IntegerOverflow {
        op: ArithmeticOp,
    },
    InvalidShiftCount {
        count: i64,
    },
    NonFiniteFloat,
    PrecisionLoss,
    UnknownVariable(String),
    UnexpectedOpcode,
    InvalidExpression,
}
```

This makes negative tests precise:

```rust
assert_eq!(
    evaluate("1 / 0", &variables),
    Err(Error::Eval(EvalError::DivisionByZero))
);
```

---

## Phase 4: Add resource limits

Goal: Hostile input should fail cheaply.

### 4.1 Add evaluation options

```rust
#[derive(Clone, Debug)]
pub struct EvalOptions {
    pub max_input_bytes: usize,
    pub max_tokens: usize,
    pub max_ast_nodes: usize,
    pub max_depth: usize,
    pub max_function_args: usize,
    pub max_parser_recoveries: usize,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            max_input_bytes: 16 * 1024,
            max_tokens: 4096,
            max_ast_nodes: 4096,
            max_depth: 256,
            max_function_args: 256,
            max_parser_recoveries: 64,
        }
    }
}
```

Add:

```rust
pub fn evaluate_with_options(
    text: &str,
    variables: &HashMap<String, Value>,
    options: &EvalOptions,
) -> Result<Value, Error>;
```

### 4.2 Enforce input byte length before lexing

```rust
if text.len() > options.max_input_bytes {
    return Err(Error::ResourceLimit(ResourceLimitError::InputTooLarge {
        actual: text.len(),
        max: options.max_input_bytes,
    }));
}
```

### 4.3 Count tokens

Wrap the lexer with a counting adapter. Stop after `max_tokens`.

### 4.4 Count AST nodes and depth

After parse, validate AST size and depth before evaluation.

```rust
fn validate_ast_limits(expr: &Expression<'_>, options: &EvalOptions) -> Result<(), Error> {
    let mut nodes = 0;
    let mut max_depth_seen = 0;

    fn walk(
        expr: &Expression<'_>,
        depth: usize,
        nodes: &mut usize,
        max_depth_seen: &mut usize,
        options: &EvalOptions,
    ) -> Result<(), Error> {
        *nodes += 1;
        *max_depth_seen = (*max_depth_seen).max(depth);

        if *nodes > options.max_ast_nodes {
            return Err(Error::ResourceLimit(ResourceLimitError::AstTooLarge));
        }

        if depth > options.max_depth {
            return Err(Error::ResourceLimit(ResourceLimitError::ExpressionTooDeep));
        }

        match expr {
            Expression::UnaryOperation { value, .. } => {
                walk(value, depth + 1, nodes, max_depth_seen, options)?;
            }
            Expression::BinaryOperation { lhs, rhs, .. } => {
                walk(lhs, depth + 1, nodes, max_depth_seen, options)?;
                walk(rhs, depth + 1, nodes, max_depth_seen, options)?;
            }
            Expression::Function { arguments, .. } => {
                if arguments.len() > options.max_function_args {
                    return Err(Error::ResourceLimit(ResourceLimitError::TooManyFunctionArguments));
                }

                for arg in arguments {
                    walk(arg, depth + 1, nodes, max_depth_seen, options)?;
                }
            }
            Expression::Bool(_)
            | Expression::Integer(_)
            | Expression::Float(_)
            | Expression::Variable(_)
            | Expression::LexicalError(_)
            | Expression::Error => {}
        }

        Ok(())
    }

    walk(expr, 0, &mut nodes, &mut max_depth_seen, options)
}
```

### 4.5 Add resource tests

Test cases:

- Input longer than `max_input_bytes`.
- More tokens than `max_tokens`.
- Deeply nested parentheses.
- Deep unary chains.
- Very large function argument lists.
- Parser recovery storms from long invalid input.

---

# Testing plan

## Required test layers

### Unit tests

Keep current parser/evaluator tests, but add boundary tests for:

- Integer min/max.
- Overflow cases.
- Division by zero.
- Modulo/remainder by zero.
- Negative shift counts.
- Oversized shift counts.
- Float domain errors.
- Non-finite float generation.
- Mixed int/float precision-loss cases.
- Approximate equality with negative and near-zero values.
- Error variant stability.

### Panic-free hostile-input tests

All hostile-input tests should use the public API and assert no panic.

```rust
let result = std::panic::catch_unwind(|| {
    let _ = evaluate(input, &variables);
});
assert!(result.is_ok());
```

### Property tests

Add:

```toml
[dev-dependencies]
proptest = "1"
```

Properties:

1. Any UTF-8 input up to N bytes never panics.
2. Any generated valid expression up to depth D never panics.
3. Integer arithmetic agrees with checked Rust semantics.
4. Division/modulo/remainder by zero returns a typed error.
5. Invalid shift counts return a typed error.
6. Any successful float result is finite.
7. Whitespace insertion does not change meaning.
8. Parenthesized expression preserves value where expected.
9. Parser diagnostics are deterministic.

Example smoke property:

```rust
proptest::proptest! {
    #[test]
    fn arbitrary_utf8_does_not_panic(input in "\\PC{0,4096}") {
        let variables = std::collections::HashMap::new();

        let result = std::panic::catch_unwind(|| {
            let _ = shunting_yard::evaluate(&input, &variables);
        });

        prop_assert!(result.is_ok());
    }
}
```

### Fuzz tests

Add `cargo-fuzz`.

Initial target:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let variables = HashMap::new();
        let _ = shunting_yard::evaluate(input, &variables);
    }
});
```

Seed corpus:

```text
9223372036854775807 + 1
9223372036854775807 * 2
1 / 0
1 % 0
mod(1, 0)
rem(1, 0)
1 << -1
1 << 64
ln(-1)
acos(2)
asin(2)
exp(10000)
1e308 * 1e308
((((((((((((((((((((1))))))))))))))))))))
min()
min(1,2,3,4,5,6,7,8,9,10)
$
1 + $
```

Add a second grammar-aware fuzzer later that generates valid expression trees, then renders them to source text.

### Miri

Miri is especially important if unsafe code, FFI, custom allocation, or generated runtime code is added later. Add it early.

```bash
rustup +nightly component add miri
cargo +nightly miri test
```

Miri is not a proof of no undefined behavior, but it is a valuable dynamic check for Rust UB classes.

### Sanitizers

Sanitizers are most valuable when unsafe, FFI, native dependencies, custom buffers, or concurrent code enter the crate. Add them to CI/nightly so the harness exists before those risks arrive.

AddressSanitizer:

```bash
RUSTFLAGS="-Zsanitizer=address" \
cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu
```

LeakSanitizer:

```bash
RUSTFLAGS="-Zsanitizer=leak" \
cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu
```

ThreadSanitizer, when concurrency is added:

```bash
RUSTFLAGS="-Zsanitizer=thread" \
cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu
```

Do not expect sanitizers to catch unchecked arithmetic or bad numeric semantics. Those are logic bugs and need tests/properties.

### Dependency checks

Add:

```bash
cargo install --locked cargo-audit
cargo audit
```

Add `cargo-deny`:

```bash
cargo install --locked cargo-deny
cargo deny init
cargo deny check
```

Track:

- security advisories
- banned crates
- duplicate versions
- unknown licenses
- unexpected git/path dependencies

---

# Lint and CI policy

## Cargo lint configuration

Add to `Cargo.toml` once the crate is ready to enforce these:

```toml
[lints.rust]
unsafe_code = "forbid"
unsafe_op_in_unsafe_fn = "deny"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
cast_possible_truncation = "deny"
cast_possible_wrap = "deny"
cast_precision_loss = "warn"
arithmetic_side_effects = "warn"
```

Generated parser code may need narrow lint allowances. Keep those allowances local to generated modules.

## Suggested CI commands

Minimum PR checks:

```bash
cargo fmt --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --release --all-targets --all-features
cargo audit
cargo deny check
```

Nightly or scheduled checks:

```bash
cargo +nightly miri test
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu
cargo fuzz run evaluate_no_panic
```

---

# Acceptance criteria

Before considering the crate high-security-ready for hostile inputs, require the following.

## Public API safety

- [ ] `evaluate(...)` does not panic for arbitrary UTF-8 input.
- [ ] `evaluate(...)` does not panic for hostile variable bindings.
- [ ] All expected invalid inputs return structured errors.
- [ ] No non-finite float can be returned from successful evaluation.
- [ ] Integer overflow returns an error.
- [ ] Invalid shift counts return an error.
- [ ] Division/modulo/remainder by zero returns an error.
- [ ] Resource limits are enforced.

## Numeric semantics

- [ ] Integer arithmetic uses checked operations.
- [ ] Float operations validate finite outputs.
- [ ] Mixed int/float semantics are documented and tested.
- [ ] Approximate equality handles negative and near-zero values.
- [ ] Float-to-int conversion has explicit boundary tests.

## Parser and diagnostics

- [ ] Parse and evaluation can be tested separately.
- [ ] Parser recovery diagnostics are preserved.
- [ ] Lexical, parse, eval, and resource-limit errors are distinguishable.
- [ ] Error variants are stable enough for callers to match.

## Testing

- [ ] Unit tests cover boundary values.
- [ ] Negative tests assert exact structured errors.
- [ ] Hostile-input tests assert no panic.
- [ ] Property tests cover arbitrary UTF-8 and generated valid expressions.
- [ ] Fuzz target exists for `evaluate`.
- [ ] Miri runs in scheduled CI.
- [ ] Sanitizer jobs exist in scheduled CI.
- [ ] Release-mode tests run in CI.

## Supply chain and linting

- [ ] `unsafe_code = "forbid"` is enabled.
- [ ] `unwrap`, `expect`, `panic`, `todo`, and `unimplemented` are denied in non-test code.
- [ ] `cargo audit` runs in CI.
- [ ] `cargo deny check` runs in CI.
- [ ] All lint allowances are narrow and documented.

---

# Suggested issue breakdown

## Issue 1: Replace unchecked evaluator arithmetic with checked operations

Implement checked helpers for integer add, subtract, multiply, negate, divide, modulo, remainder, and power. Update evaluator call sites. Add regression tests for integer boundary cases.

Acceptance:

- No primitive integer arithmetic remains in evaluator except through checked helper functions.
- Public `evaluate` does not panic on integer boundary inputs.
- Debug and release tests agree.

## Issue 2: Validate bitshift counts

Implement checked shift-count conversion and use `checked_shl` / `checked_shr`.

Acceptance:

- Negative shifts return `InvalidShiftCount`.
- Shifts greater than or equal to `i64::BITS` return `InvalidShiftCount`.
- Valid shifts work.

## Issue 3: Enforce finite float invariant

Introduce `FiniteF64` and validate all float-producing operations.

Acceptance:

- Float literals still reject NaN/Inf/subnormal values if that remains desired policy.
- Evaluator cannot return NaN/Inf.
- Domain errors such as `ln(-1)` and `acos(2)` return structured errors.

## Issue 4: Replace stringly typed errors

Replace `InvalidType(String)` and `MathError(String)` with structured variants.

Acceptance:

- Negative tests match exact variants.
- User-facing display messages are still available through `Display`.
- Internal tests do not depend on free-form strings.

## Issue 5: Add resource limits

Add `EvalOptions` and enforce input size, token count, AST depth, AST node count, function argument count, and parser recovery count.

Acceptance:

- Deeply nested input fails with a resource-limit error.
- Huge function argument lists fail with a resource-limit error.
- Invalid-input storms fail cheaply.

## Issue 6: Split parse and eval APIs

Expose or internally separate `parse`, `evaluate_ast`, and `evaluate`.

Acceptance:

- Parser tests do not need to invoke evaluator.
- Evaluator tests can construct ASTs directly.
- Parse diagnostics are preserved.

## Issue 7: Add property tests and fuzzing

Add `proptest` and `cargo-fuzz` targets.

Acceptance:

- Arbitrary UTF-8 input does not panic.
- Generated valid expressions do not panic.
- Successful float results are finite.
- Fuzz corpus includes integer boundary, float boundary, invalid syntax, and resource-limit cases.

## Issue 8: Add high-security CI

Add format, check, clippy, test, release test, audit, deny, Miri, sanitizer, and fuzz jobs.

Acceptance:

- PR CI catches formatting, lint, test, audit, and deny failures.
- Scheduled CI runs Miri, sanitizer jobs, and fuzzing.
- Generated parser code has only narrow documented lint exceptions.

---

# Implementation priority

Do these first:

1. Checked arithmetic.
2. Shift validation.
3. Division/modulo/remainder zero and overflow checks.
4. Panic-free hostile-input tests.
5. Structured errors.
6. Finite float wrapper.
7. Resource limits.
8. Property tests.
9. Fuzzing.
10. Miri/sanitizer/dependency CI.

Avoid adding more expression-language features until the evaluator no longer panics and the numeric invariants are enforced.
