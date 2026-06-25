#![no_main]

use libfuzzer_sys::fuzz_target;
use shunting_yard::evaluate;
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let variables = HashMap::new();
        let _ = evaluate(input, &variables);
    }
});
