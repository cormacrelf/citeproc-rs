# ffi interface to `citeproc-rs`

```sh
# prerequisite
cargo install --force cbindgen

# from this ffi directory
make

# or more explicitly
cargo build
cbindgen -o include/citeproc_rs.h
make ./examples/client

# linked to to the dylib in <repo root>/target/debug/libciteproc_rs.dylib
./examples/client
```
