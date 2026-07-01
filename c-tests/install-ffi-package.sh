#!/usr/bin/env bash
set -euo pipefail

profile="${PROFILE:-debug}"
prefix="${1:-target/shunting-yard-ffi-install}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "$profile" in
  debug)
    cargo_args=()
    target_dir="debug"
    ;;
  release)
    cargo_args=(--release)
    target_dir="release"
    ;;
  *)
    echo "unsupported PROFILE: $profile" >&2
    exit 2
    ;;
esac

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$repo_root/Cargo.toml" | head -n 1)"
if [ -z "$version" ]; then
  echo "could not determine workspace package version" >&2
  exit 2
fi

cargo build -p shunting_yard_ffi "${cargo_args[@]}"

prefix_abs="$(mkdir -p "$prefix" && cd "$prefix" && pwd)"
include_dir="$prefix_abs/include"
lib_dir="$prefix_abs/lib"
pkgconfig_dir="$lib_dir/pkgconfig"
cmake_dir="$lib_dir/cmake/ShuntingYardFFI"
artifact_dir="$repo_root/target/$target_dir"

mkdir -p "$include_dir" "$lib_dir" "$pkgconfig_dir" "$cmake_dir"

cp "$repo_root/crates/shunting-yard-ffi/include/shunting_yard_ffi.h" "$include_dir/"

for lib_name in \
  libshunting_yard_ffi.a \
  libshunting_yard_ffi.so \
  libshunting_yard_ffi.dylib \
  shunting_yard_ffi.dll \
  shunting_yard_ffi.lib
do
  if [ -f "$artifact_dir/$lib_name" ]; then
    cp "$artifact_dir/$lib_name" "$lib_dir/"
  fi
done

cat > "$pkgconfig_dir/shunting_yard_ffi.pc" <<EOF
prefix=$prefix_abs
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include

Name: shunting_yard_ffi
Description: C ABI for the Rust shunting-yard expression evaluator
Version: $version
Cflags: -I\${includedir}
Libs: -L\${libdir} -lshunting_yard_ffi
Libs.private: -ldl -lpthread -lm
EOF

cat > "$cmake_dir/ShuntingYardFFIConfig.cmake" <<EOF
include("\${CMAKE_CURRENT_LIST_DIR}/ShuntingYardFFITargets.cmake")

set(ShuntingYardFFI_VERSION "$version")
set(ShuntingYardFFI_INCLUDE_DIR "\${_SHUNTING_YARD_FFI_PREFIX}/include")
set(ShuntingYardFFI_LIBRARY_DIR "\${_SHUNTING_YARD_FFI_PREFIX}/lib")
EOF

cat > "$cmake_dir/ShuntingYardFFITargets.cmake" <<'EOF'
get_filename_component(_SHUNTING_YARD_FFI_PREFIX "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)
set(_SHUNTING_YARD_FFI_INCLUDE_DIR "${_SHUNTING_YARD_FFI_PREFIX}/include")
set(_SHUNTING_YARD_FFI_LIBRARY_DIR "${_SHUNTING_YARD_FFI_PREFIX}/lib")

if(NOT TARGET ShuntingYardFFI::ShuntingYardFFIShared)
  if(WIN32)
    set(_SHUNTING_YARD_FFI_SHARED "${_SHUNTING_YARD_FFI_LIBRARY_DIR}/shunting_yard_ffi.dll")
  elseif(APPLE)
    set(_SHUNTING_YARD_FFI_SHARED "${_SHUNTING_YARD_FFI_LIBRARY_DIR}/libshunting_yard_ffi.dylib")
  else()
    set(_SHUNTING_YARD_FFI_SHARED "${_SHUNTING_YARD_FFI_LIBRARY_DIR}/libshunting_yard_ffi.so")
  endif()

  if(EXISTS "${_SHUNTING_YARD_FFI_SHARED}")
    add_library(ShuntingYardFFI::ShuntingYardFFIShared SHARED IMPORTED)
    set_target_properties(ShuntingYardFFI::ShuntingYardFFIShared PROPERTIES
      IMPORTED_LOCATION "${_SHUNTING_YARD_FFI_SHARED}"
      INTERFACE_INCLUDE_DIRECTORIES "${_SHUNTING_YARD_FFI_INCLUDE_DIR}"
    )
  endif()
endif()

if(NOT TARGET ShuntingYardFFI::ShuntingYardFFIStatic)
  set(_SHUNTING_YARD_FFI_STATIC "${_SHUNTING_YARD_FFI_LIBRARY_DIR}/libshunting_yard_ffi.a")

  if(EXISTS "${_SHUNTING_YARD_FFI_STATIC}")
    add_library(ShuntingYardFFI::ShuntingYardFFIStatic STATIC IMPORTED)
    set_target_properties(ShuntingYardFFI::ShuntingYardFFIStatic PROPERTIES
      IMPORTED_LOCATION "${_SHUNTING_YARD_FFI_STATIC}"
      INTERFACE_INCLUDE_DIRECTORIES "${_SHUNTING_YARD_FFI_INCLUDE_DIR}"
    )

    if(UNIX AND NOT APPLE)
      set_property(TARGET ShuntingYardFFI::ShuntingYardFFIStatic APPEND PROPERTY
        INTERFACE_LINK_LIBRARIES dl pthread m
      )
    endif()
  endif()
endif()

if(NOT TARGET ShuntingYardFFI::ShuntingYardFFI)
  if(TARGET ShuntingYardFFI::ShuntingYardFFIShared)
    add_library(ShuntingYardFFI::ShuntingYardFFI ALIAS ShuntingYardFFI::ShuntingYardFFIShared)
  elseif(TARGET ShuntingYardFFI::ShuntingYardFFIStatic)
    add_library(ShuntingYardFFI::ShuntingYardFFI ALIAS ShuntingYardFFI::ShuntingYardFFIStatic)
  else()
    message(FATAL_ERROR "ShuntingYardFFI package found, but no supported library artifact exists")
  endif()
endif()
EOF

printf '%s\n' "$prefix_abs"
