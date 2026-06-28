#![no_main]

use libfuzzer_sys::fuzz_target;
use shunting_yard::{Value, evaluate};
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let variables = HashMap::from([
        ("a".to_string(), Value::Integer(1)),
        ("b".to_string(), Value::Integer(2)),
        ("x".to_string(), Value::Float(3.5)),
        ("flag".to_string(), Value::Bool(true)),
    ]);

    let _ = evaluate(&text, &variables);
});
