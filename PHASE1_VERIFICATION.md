
## Final verification (performed by the main agent)

- `cargo clean && cargo build --release` → **green**, zero warnings.
  Only one build configuration exists (`[features]` is just `default = []`).
- Exported-symbol diff vs. the reference C library built from `c_src/src/*.c`
  with `gcc -O2 -DHAVE_CONFIG_H -DNDEBUG -shared -fPIC`:
  **61 symbols on each side, zero differences in either direction.**
- Struct size/alignment/field-offset equality with the gcc-compiled `yaml.h` is
  enforced by `const _: () = assert!(...)` blocks at the bottom of `src/lib.rs`,
  so any future layout drift is a compile error.
- Smoke comparison (reference C .so vs. Rust .so, same C harness sources
  compiled twice, identical argv[0] via sibling dirs A/ and B/):
  **462 comparisons, 0 differences.** Covered all 14 programs in `c_src/tests/`
  (run-parser, run-scanner, run-loader, run-dumper, run-emitter,
  run-parser-test-suite, run-emitter-test-suite, test-version, test-reader,
  test-nesting, example-reformatter(-alt), example-deconstructor(-alt)) over
  every file in `c_src/examples/`, `c_src/regression-inputs/`, every
  `c_src/tests/*.c`, `yaml.h` and a couple of markdown files used as arbitrary
  input, with the `-c` / `-u` / `-c -u` option combinations where applicable.
  stdout, stderr and exit status all matched byte-for-byte.
- `c_src/` content is unmodified (`git diff` shows only pre-existing
  executable-bit mode changes on three shell scripts, 0 insertions/deletions).
- `Cargo.toml`, `rust-toolchain.toml` untouched; no `build.rs` was created; no
  dependencies were added (the crate has none).
