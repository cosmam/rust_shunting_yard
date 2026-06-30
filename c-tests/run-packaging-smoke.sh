#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
prefix="$("$repo_root/c-tests/install-ffi-package.sh" "$repo_root/target/ffi-package/install")"
pkg_config_path="$prefix/lib/pkgconfig"
example_source="$repo_root/examples/c-consumer/main.c"

command -v pkg-config >/dev/null
command -v cmake >/dev/null
command -v cc >/dev/null

pkg_cflags="$(PKG_CONFIG_PATH="$pkg_config_path" pkg-config --cflags shunting_yard_ffi)"
pkg_libs="$(PKG_CONFIG_PATH="$pkg_config_path" pkg-config --libs shunting_yard_ffi)"
PKG_CONFIG_PATH="$pkg_config_path" pkg-config --libs --static shunting_yard_ffi >/dev/null

cc \
  -std=c11 \
  -Wall \
  -Wextra \
  -Werror \
  $pkg_cflags \
  "$example_source" \
  $pkg_libs \
  -o "$repo_root/target/ffi-package/pkg-config-consumer"

LD_LIBRARY_PATH="$prefix/lib:${LD_LIBRARY_PATH:-}" \
  "$repo_root/target/ffi-package/pkg-config-consumer"

cc \
  -std=c11 \
  -Wall \
  -Wextra \
  -Werror \
  -I "$prefix/include" \
  "$example_source" \
  -L "$prefix/lib" \
  -lshunting_yard_ffi \
  -o "$repo_root/target/ffi-package/shared-consumer"

LD_LIBRARY_PATH="$prefix/lib:${LD_LIBRARY_PATH:-}" \
  "$repo_root/target/ffi-package/shared-consumer"

cc \
  -std=c11 \
  -Wall \
  -Wextra \
  -Werror \
  -I "$prefix/include" \
  "$example_source" \
  "$prefix/lib/libshunting_yard_ffi.a" \
  -ldl \
  -lpthread \
  -lm \
  -o "$repo_root/target/ffi-package/static-consumer"

"$repo_root/target/ffi-package/static-consumer"

cmake \
  -S "$repo_root/examples/c-consumer" \
  -B "$repo_root/target/ffi-package/cmake-consumer" \
  -DCMAKE_PREFIX_PATH="$prefix"
cmake --build "$repo_root/target/ffi-package/cmake-consumer"
LD_LIBRARY_PATH="$prefix/lib:${LD_LIBRARY_PATH:-}" \
  "$repo_root/target/ffi-package/cmake-consumer/shunting_yard_ffi_c_consumer"

cmake \
  -S "$repo_root/examples/c-consumer" \
  -B "$repo_root/target/ffi-package/cmake-consumer-static" \
  -DCMAKE_PREFIX_PATH="$prefix" \
  -DSHUNTING_YARD_FFI_LINK_STATIC=ON
cmake --build "$repo_root/target/ffi-package/cmake-consumer-static"
"$repo_root/target/ffi-package/cmake-consumer-static/shunting_yard_ffi_c_consumer"

printf '%s\n' "shunting_yard_ffi packaging smoke test passed"
