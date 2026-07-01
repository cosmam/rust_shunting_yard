A repo for me to play around with implementations of the Shunting Yard Algorithm! And possibly related cases, depending on how long my attention holds.

## Background Assumptions

Any constants have already been replaced with their numerical values. So, for instance, neither pi nor π will be in the equation to evaluate; it will already have been replaced with 3.14159

Whitespace has no effect.

## Workspace Layout

- `crates/shunting-yard`: safe Rust expression parser/evaluator.
- `crates/shunting-yard-ffi`: C ABI adapter exposing no-variable evaluation,
  callback-backed variable resolution, and optional owned error objects.
- `c-tests`: external C smoke test for the FFI library.

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

### Parse once, evaluate many times

For callers that need to evaluate the same expression against different variable
sources, parse the expression once and evaluate the parsed expression
repeatedly:

```rust
use shunting_yard::{evaluate_parsed, parse, EvalError, Value};
use std::collections::HashMap;

let parsed = parse("x + 1")?;

let mut first = HashMap::new();
first.insert("x".to_owned(), Value::Integer(1));

let mut second = HashMap::new();
second.insert("x".to_owned(), Value::Integer(41));

assert_eq!(evaluate_parsed(&parsed, &first), Ok(Value::Integer(2)));
assert_eq!(evaluate_parsed(&parsed, &second), Ok(Value::Integer(42)));

# Ok::<(), EvalError>(())
```

`parse_with_options` enforces input, token, parser recovery, AST node, depth,
and function-argument limits before evaluation. `evaluate_parsed` still
validates resolver-returned values before using them.

### Diagnostic-aware APIs

For callers that need to distinguish lexical, parse, resource-limit, and
evaluation failures, use the diagnostic-aware APIs. The original `evaluate`,
`parse`, and parsed-evaluation APIs remain available for compatibility.

```rust
use shunting_yard::{evaluate_detailed, Error};
use std::collections::HashMap;

let variables = HashMap::new();

match evaluate_detailed("1 / 0", &variables) {
    Err(Error::Eval(error)) => {
        println!("evaluation failed: {error}");
    }
    Err(Error::Parse(diagnostics)) => {
        println!("parse failed with {} diagnostic(s)", diagnostics.len());
    }
    Err(Error::Lexical { span, error }) => {
        println!("lexical error at {span:?}: {error}");
    }
    Err(Error::ResourceLimit(error)) => {
        println!("resource limit exceeded: {error}");
    }
    Ok(value) => {
        println!("value: {value:?}");
    }
}
```

Detailed parser diagnostics include source spans when available and preserve
parser recovery information separately from unrecovered parse failures.

All evaluation paths reject variable values and operation results that would
produce NaN, infinity, or subnormal floating-point values.

## FFI API

The FFI crate exposes simple status-only entrypoints:

- `shy_evaluate_no_vars`
- `shy_evaluate_no_vars_with_options`
- `shy_evaluate_with_callback`
- `shy_evaluate_with_callback_with_options`
- `shy_parse_expression`
- `shy_parse_expression_with_options`
- `shy_evaluate_parsed_no_vars`
- `shy_evaluate_parsed_with_callback`

It also exposes extended `_ex` entrypoints that can return an owned `ShyError`
through `ShyError **out_error`:

- `shy_evaluate_no_vars_ex`
- `shy_evaluate_no_vars_with_options_ex`
- `shy_evaluate_with_callback_ex`
- `shy_evaluate_with_callback_with_options_ex`
- `shy_parse_expression_ex`
- `shy_parse_expression_with_options_ex`
- `shy_evaluate_parsed_no_vars_ex`
- `shy_evaluate_parsed_with_callback_ex`

Passing `out_error = NULL` is allowed and disables error object allocation. If
an error object is returned, the C caller owns it and must release it with
`shy_error_free`. Pointers returned by `shy_error_message` are borrowed from
the error object and remain valid only until `shy_error_free`.

Parse-stage `ShyError` objects can also be inspected through indexed diagnostic
accessors. These expose diagnostic kind, source span, and expected-token strings
where available. Expected-token strings are borrowed from the error object and
are intended for diagnostics/display rather than as a stable grammar schema.

For configurable resource limits, initialize `ShyEvalOptions` with
`shy_eval_options_default`, adjust nonzero limits as needed, and pass it to the
`_with_options` entrypoints. Parsed-handle evaluation does not take options
because parse-time limits are enforced when the handle is created.

The FFI crate also exposes opaque parsed-expression handles. C callers can
parse once, evaluate the handle repeatedly with no variables or callback-backed
variables, and release the handle with `shy_parsed_expression_free`.

### FFI Packaging

The FFI package installer creates a native-consumer layout:

```text
include/shunting_yard_ffi.h
lib/libshunting_yard_ffi.a
lib/libshunting_yard_ffi.so
lib/pkgconfig/shunting_yard_ffi.pc
lib/cmake/ShuntingYardFFI/ShuntingYardFFIConfig.cmake
lib/cmake/ShuntingYardFFI/ShuntingYardFFITargets.cmake
```

Build and install the debug package with:

```bash
c-tests/install-ffi-package.sh target/ffi-package/install
```

For optimized artifacts:

```bash
PROFILE=release c-tests/install-ffi-package.sh target/ffi-package/install
```

pkg-config consumers can use:

```bash
PKG_CONFIG_PATH="$(pwd)/target/ffi-package/install/lib/pkgconfig" \
  pkg-config --cflags --libs shunting_yard_ffi
```

CMake consumers can use:

```cmake
find_package(ShuntingYardFFI REQUIRED)
target_link_libraries(app PRIVATE ShuntingYardFFI::ShuntingYardFFI)
```

The package also provides `ShuntingYardFFI::ShuntingYardFFIShared` and
`ShuntingYardFFI::ShuntingYardFFIStatic` imported targets when the matching
library artifact is installed.

On Linux, shared-library consumers need the installed `lib/` directory on the
runtime loader path, for example through `LD_LIBRARY_PATH`, an rpath, or a
system library path. Static-library consumers link `libshunting_yard_ffi.a`;
the provided CMake target adds the Linux system libraries needed by Rust's
static runtime. On macOS, use the installed `.dylib` and configure the runtime
search path with `DYLD_LIBRARY_PATH`, an rpath, or the app bundle layout. On
Windows, ship the DLL next to the executable or on `PATH`, and link through the
import library produced by the Rust build.

The C ABI version follows the crate workspace version. Function names, status
codes, error codes, callback signatures, `ShyValue`, and `ShyEvalOptions` are
covered by ABI smoke tests to catch accidental changes.

See [examples/c-consumer](examples/c-consumer) for a standalone CMake consumer
that exercises parsing, callback evaluation, resource limits, error reporting,
and parsed-expression reuse.

## Verification

The main local verification pass is:

```bash
cargo fmt --check
cargo test --workspace --all-targets --all-features
cargo test --release --workspace --all-targets --all-features
cargo test --doc --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./c-tests/run-smoke.sh
./c-tests/run-packaging-smoke.sh
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
