#!/usr/bin/env bash
set -euo pipefail

cargo build -p shunting_yard_ffi

cc \
  -std=c11 \
  -Wall \
  -Wextra \
  -Werror \
  -I crates/shunting-yard-ffi/include \
  c-tests/smoke.c \
  -L target/debug \
  -lshunting_yard_ffi \
  -o target/debug/shunting_yard_ffi_smoke

LD_LIBRARY_PATH="target/debug:${LD_LIBRARY_PATH:-}" \
  target/debug/shunting_yard_ffi_smoke

cc \
  -std=c11 \
  -Wall \
  -Wextra \
  -Werror \
  -I crates/shunting-yard-ffi/include \
  c-tests/abi.c \
  -L target/debug \
  -lshunting_yard_ffi \
  -o target/debug/shunting_yard_ffi_abi

LD_LIBRARY_PATH="target/debug:${LD_LIBRARY_PATH:-}" \
  target/debug/shunting_yard_ffi_abi
