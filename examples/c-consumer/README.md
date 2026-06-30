# Shunting Yard FFI C Consumer

This standalone CMake project consumes an installed `shunting_yard_ffi`
package. It does not depend on Cargo or the Rust crate layout after the FFI
package has been installed.

From the repository root:

```bash
c-tests/install-ffi-package.sh target/ffi-package/install
cmake -S examples/c-consumer -B target/ffi-package/cmake-consumer \
  -DCMAKE_PREFIX_PATH="$(pwd)/target/ffi-package/install"
cmake --build target/ffi-package/cmake-consumer
LD_LIBRARY_PATH="$(pwd)/target/ffi-package/install/lib:${LD_LIBRARY_PATH:-}" \
  target/ffi-package/cmake-consumer/shunting_yard_ffi_c_consumer
```

To force the static imported target:

```bash
cmake -S examples/c-consumer -B target/ffi-package/cmake-consumer-static \
  -DCMAKE_PREFIX_PATH="$(pwd)/target/ffi-package/install" \
  -DSHUNTING_YARD_FFI_LINK_STATIC=ON
cmake --build target/ffi-package/cmake-consumer-static
target/ffi-package/cmake-consumer-static/shunting_yard_ffi_c_consumer
```
