# Translation Plan

## Self-assessment
- Model: Claude Opus 5 (claude-opus-5[1m])
- Window: ~1,000,000 tokens usable
- Output token limit: ~32k tokens per single response (conservative working number; size
  every sub-agent subtask so the Rust it writes in one response stays under ~900 lines)
- Regime: **input-small / output-large**. Total C library source
  (`c_src/src/*.c` + `c_src/src/yaml_private.h` + `c_src/include/yaml.h`) is ~12.5k lines
  ≈ 105k tokens ≈ 10% of window, so reading is cheap. But the Rust output is ~15k lines,
  which blows past any single-response output limit. => Treat as **large**: PLAN.md driven,
  aggressive sub-agent delegation, one module per subtask.

## Invariants (do not drift across compactions)

These rules are not negotiable and must survive every compaction unchanged.
When in doubt, re-read this section.

### AFTER ANY COMPACTION: `cat PLAN.md` is your FIRST action before anything.

### Cargo features and the provided scaffold
- The `Cargo.toml`, `build.rs`, and `rust-toolchain.toml` in the project root
  are **provided** and must NOT be modified or recreated. In particular, the
  `[features]` block (one feature per configurable variable value) and the
  `build.rs` that maps selected features to `cfg`s are already written for you.
- For a configurable variable `VAR` with values `a`, `b`, the provided features
  are named `VAR_a`, `VAR_b` (the variable name, an underscore, then the value).
  Gate code with the **bare cfg** form `#[cfg(VAR_a)]` (NOT `#[cfg(feature =
  "VAR_a")]`). For a boolean variable `VAR`, gate with `#[cfg(feature = "VAR")]`.
  Use the exact feature names as written in the provided `Cargo.toml`; do not
  invent, rename, or add features.
- Emit a gated definition for EVERY value of each variable — do NOT hardcode a
  single default, and do NOT read a variable's value via `env!(...)`.
- ALL feature combinations must compile (`cargo build --release --features <combo>`).

### Cargo manifest target names
- The provided `Cargo.toml` already sets the `[lib]`/`[[bin]]` `name`. Do not
  change it. (It uses underscores only — hyphens would break manifest parsing.)
  Match your `src/` layout to it: `src/main.rs` for a bin, `src/lib.rs` for a lib.

### C ABI
- Public C exports use `#[unsafe(no_mangle)]` and `extern "C"` with exact C
  signatures (use `*const c_char`, `c_int`, etc. from `std::ffi`).
- The exported symbol name is the FINAL linker symbol after all preprocessor
  renames. If C has `#define foo NAMESPACE(foo)` producing `PREFIX_foo`, the
  Rust export is named `PREFIX_foo`, not `foo`.

### Behavioral fidelity
- Do NOT fix bugs in the original C. Reproduce behavior exactly.
- Preserve the exact order of error checks and validation.
- Match C's stdin reading semantics (scanf reads across newlines; fgets does not).
- Match C's exact printf format including spacing and newlines.

### Crate constraints
- Do NOT use the `openssl` crate or any OpenSSL bindings. Use pure-Rust crates.
- Prefer safe Rust internally; do not relax the C ABI on exports.

### Boundaries
- Do NOT modify anything in `c_src/`.

### Translation fidelity
- You MUST faithfully translate ALL C source files to **pure Rust**. Do NOT use
  the `cc` crate (or any equivalent) in `build.rs` to compile or link the
  original C source code. The C source files in `c_src/` will NOT exist in
  the final test environment — the only code available at test time is the
  Rust you write. Any attempt to wrap C via a compiled static archive or
  object file will fail at test time because the C files simply won't be
  there.
- Do NOT import or depend on any existing Rust crate that implements, wraps,
  re-exports, or compiles the same C library you are translating. Every line
  of Rust code must be written by you. If a function needs to call out to
  system libraries (e.g. POSIX APIs), use `libc` or equivalent thin FFI crates,
  not crates that compile the library you are meant to translate.
- A `build.rs` is allowed for legitimate build-time needs (code generation,
  feature detection, etc.), but it must NOT reference, compile, or link any
  file under `c_src/`.
- No shortcuts: every function, every struct, every constant, every macro in
  the C source must become a corresponding Rust implementation. Stub
  functions that return 0 or a hardcoded value are NOT acceptable unless the
  function's return value is defined by the API contract as a compile-time
  constant.

## Project-specific invariants (libyaml -> libyamlrs)

1. **No external crates may be added.** The provided `Cargo.toml` has NO
   `[dependencies]` section and must not be modified. Therefore `libc` is NOT
   available. All libc functions (`malloc`, `realloc`, `free`, `strdup`,
   `memset`, `memcpy`, `memmove`, `strlen`, `strcmp`, `fread`, `fwrite`,
   `fflush`, `ferror`) are declared by hand in `src/libc_shim.rs` with
   `extern "C" { ... }`. A `cdylib` links libc on Linux, so these resolve.
2. **Memory must come from libc `malloc`/`free`**, never Rust's allocator —
   C callers free library-allocated strings and the library frees
   `strdup`-allocated strings.
3. **Every public struct is `#[repr(C)]` with exactly the C field order.**
   C test programs allocate `yaml_parser_t` / `yaml_emitter_t` / `yaml_event_t`
   / `yaml_document_t` on their own stack and poke fields directly, so SIZE,
   ALIGNMENT and FIELD OFFSETS must match byte-for-byte.
4. **C enums are translated as `pub type X = libc_shim::c_uint;` plus `pub const`
   values**, not Rust `enum`s. Reason: C callers can store out-of-range values;
   a Rust `enum` would be instant UB. Consts also work as `match` patterns.
5. **Anonymous inner structs/unions** in `yaml.h` become named
   `#[repr(C)]` structs/unions in `src/lib.rs` following the convention
   `yaml_<outer>_<field>_t` (e.g. `yaml_parser_input_t`,
   `yaml_token_data_t`, `yaml_event_data_scalar_t`).
6. **`assert()` from C is translated to `debug_assert!`** — release builds
   define `NDEBUG`, so asserts are no-ops in the reference build.
   **APPLIES TO `c_src/src/*.c` ONLY.** In `c_src/tests/*.c` every file does
   `#ifdef NDEBUG / #undef NDEBUG / #endif` before `#include <assert.h>`, so
   there the assertions are LIVE and their arguments are always evaluated —
   see the phase-2 section at the bottom of this file.
7. **The C preprocessor macros in `yaml_private.h` become `macro_rules!`
   macros in `src/private.rs`**, exported crate-wide with
   `#[macro_export]` + `#![allow(...)]`, or re-exported via
   `pub(crate) use`. They are duck-typed on a `.pointer` / `.start` / `.end` /
   `.top` / `.head` / `.tail` field, exactly like in C.
8. ~~**Scope: the LIBRARY only**~~ **<<< REVOKED — THIS WAS WRONG. DO NOT ACT
   ON IT. See the "PHASE 2" section at the bottom of this file. >>>**
   It claimed `c_src/tests/*.c` could not be translated because `Cargo.toml`
   declares no `[[bin]]`. False premise: **Cargo auto-discovers every
   `src/bin/*.rs` as a binary target**, so all 14 test programs are translated
   there without editing `Cargo.toml`. Verified empirically.
9. **No `[features]` to gate.** The provided `Cargo.toml` has only
   `default = []`. `c_src/CMakeLists.txt`'s `BUILD_SHARED_LIBS` /
   `YAML_STATIC_LIB_NAME` options only affect linkage, not behaviour.
   `cmake/config.h.in` only supplies the version macros, which are hardcoded:
   major 0, minor 2, patch 5, string "0.2.5".
10. **Naming**: keep the C names verbatim (`yaml_parser_fetch_more_tokens`,
    `yaml_parser_scan_plain_scalar`, ...). `static` C functions become
    `pub(crate)` Rust functions so other modules can call them.
    Non-`static` C functions become `#[unsafe(no_mangle)] pub unsafe extern "C"`.
    Crate-wide `#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals,
    unused_parens, unused_assignments, dead_code)]`.

## Codebase summary

- `c_src/include/yaml.h` (2015) — public API: types, enums, structs, 60 exported fns.
- `c_src/src/yaml_private.h` (684) — internal macros (buffer/string/stack/queue,
  character classification, token/event/node initializers) + internal fn decls.
- `c_src/src/api.c` (1393) — memory mgmt, stack/queue/string helpers, parser &
  emitter init/delete, input/output setters, token/event/document constructors.
- `c_src/src/reader.c` (469) — `yaml_parser_update_buffer`, UTF-8/16 decoding.
- `c_src/src/scanner.c` (3616) — tokenizer. Biggest file.
- `c_src/src/parser.c` (1398) — event-producing state machine.
- `c_src/src/loader.c` (543) — document composer (`yaml_parser_load`).
- `c_src/src/writer.c` (141) — `yaml_emitter_flush` + encoding to raw buffer.
- `c_src/src/emitter.c` (2358) — event -> text emitter state machine.
- `c_src/src/dumper.c` (394) — `yaml_emitter_open/close/dump` serializer.
- Project type: **cdylib library only** (`libyamlrs`).
- Build configurability: **none** (no Cargo features).
- Public API surface: all `YAML_DECLARE(...)` functions in `yaml.h` *plus* the
  internal-but-exported ones declared in `yaml_private.h`
  (`yaml_malloc`, `yaml_realloc`, `yaml_free`, `yaml_strdup`,
  `yaml_string_extend`, `yaml_string_join`, `yaml_stack_extend`,
  `yaml_queue_extend`, `yaml_parser_update_buffer`,
  `yaml_parser_fetch_more_tokens`). All of those are `YAML_DECLARE` =>
  real exported symbols => `#[unsafe(no_mangle)] extern "C"`.

## Module map (Rust)

| Rust file | C origin |
|---|---|
| `src/lib.rs` | module decls + `yaml.h` types/consts |
| `src/libc_shim.rs` | hand-rolled libc `extern "C"` decls + C type aliases |
| `src/private.rs` | `yaml_private.h` constants + `macro_rules!` macros |
| `src/api.rs` | `api.c` lines 1–~640 (version, memory, helpers, parser/emitter init+setters) |
| `src/api_objects.rs` | `api.c` lines ~640–1393 (token/event/document constructors & deleters) |
| `src/reader.rs` | `reader.c` |
| `src/scanner.rs` | `scanner.c` part 1 (infrastructure + fetchers) |
| `src/scanner_tokens.rs` | `scanner.c` part 2 (fetch_* token producers) |
| `src/scanner_scan.rs` | `scanner.c` part 3 (scan_to_next_token, directives, anchor, tag) |
| `src/scanner_scalar.rs` | `scanner.c` part 4 (block/flow/plain scalar scanners) |
| `src/parser.rs` | `parser.c` part 1 |
| `src/parser_states.rs` | `parser.c` part 2 |
| `src/loader.rs` | `loader.c` |
| `src/writer.rs` | `writer.c` |
| `src/emitter.rs` | `emitter.c` part 1 (state machine) |
| `src/emitter_analyze.rs` | `emitter.c` part 2 (check_*/analyze_*) |
| `src/emitter_write.rs` | `emitter.c` part 3 (write_*) |
| `src/dumper.rs` | `dumper.c` |

## Translation subtasks

- [x] T0: Foundation A — `src/libc_shim.rs` + `src/lib.rs` (all `yaml.h` types,
      enums-as-consts, structs `#[repr(C)]`, fn-ptr typedefs, tag constants).
      ~700 Rust lines. Owner: main agent.
- [x] T1: Foundation B — `src/private.rs` (`yaml_private.h`: size constants,
      `yaml_string_t`, every macro as `macro_rules!`). ~600 Rust lines.
      Owner: main agent. Depends on T0.
- [x] T2: `src/api.rs` — `api.c` version/memory/`yaml_string_extend`/`join`/
      `stack_extend`/`queue_extend`, `yaml_parser_initialize`/`delete`,
      string/file read handlers, `yaml_parser_set_input*`, `set_encoding`,
      `yaml_emitter_initialize`/`delete`, write handlers,
      `yaml_emitter_set_output*`, `set_canonical/indent/width/unicode/break`.
      (api.c lines 1–~640). Depends on T0,T1.
- [x] T3: `src/api_objects.rs` — `api.c` remainder: `yaml_token_delete`,
      all `yaml_*_event_initialize`, `yaml_event_delete`,
      `yaml_document_initialize/delete/get_node/get_root_node/add_scalar/
      add_sequence/add_mapping/append_sequence_item/append_mapping_pair`.
      Depends on T0,T1.
- [x] T4: `src/reader.rs` — all of `reader.c`. Depends on T0,T1.
- [x] T5: `src/writer.rs` — all of `writer.c`. Depends on T0,T1.
- [x] T6: `src/scanner.rs` — `scanner.c` infra: `yaml_parser_scan`,
      `yaml_parser_fetch_more_tokens`, `yaml_parser_fetch_next_token`,
      stale-simple-keys, save/remove simple key, increase/decrease flow level,
      roll/unroll indent, set_scanner_error. Depends on T0,T1.
- [x] T7: `src/scanner_tokens.rs` — `scanner.c` `yaml_parser_fetch_*` producers
      (stream start/end, directive, document indicator, flow collection start/end,
      block/flow entry, key, value, alias, anchor, tag, block/flow/plain scalar).
      Depends on T6.
- [x] T8: `src/scanner_scan.rs` — `scanner.c` `yaml_parser_scan_to_next_token`,
      `scan_directive`, `scan_directive_name`, `scan_version_directive_value`,
      `scan_version_directive_number`, `scan_tag_directive_value`,
      `scan_anchor`, `scan_tag`, `scan_tag_handle`, `scan_tag_uri`,
      `scan_uri_escapes`. Depends on T6.
- [x] T9: `src/scanner_scalar.rs` — `scanner.c` `scan_block_scalar`,
      `scan_block_scalar_breaks`, `scan_flow_scalar`, `scan_plain_scalar`.
      Depends on T6.
- [x] T10: `src/parser.rs` — `parser.c` part 1: `yaml_parser_parse`,
      `set_parser_error*`, `state_machine`, `parse_stream_start`,
      `parse_document_start`, `parse_document_content`, `parse_document_end`,
      `parse_node`, `parse_block_sequence_entry`, `parse_indentless_sequence_entry`.
      Depends on T0,T1.
- [x] T11: `src/parser_states.rs` — `parser.c` part 2: block/flow mapping and
      sequence entry states, `parse_flow_*`, `process_empty_scalar`,
      `process_directives`, `append_tag_directive`. Depends on T10.
- [x] T12: `src/loader.rs` — all of `loader.c`. Depends on T0,T1.
- [x] T13: `src/dumper.rs` — all of `dumper.c`. Depends on T0,T1.
- [x] T14: `src/emitter.rs` — `emitter.c` part 1: `yaml_emitter_emit`,
      `set_emitter_error`, `need_more_events`, `append_tag_directive`,
      `increase_indent`, `state_machine` + all `yaml_emitter_emit_*` state fns.
      Depends on T0,T1.
- [x] T15: `src/emitter_analyze.rs` — `emitter.c` part 2: `check_empty_document`,
      `check_empty_sequence`, `check_empty_mapping`, `check_simple_key`,
      `select_scalar_style`, `process_anchor`, `process_tag`, `process_scalar`,
      `analyze_version_directive`, `analyze_tag_directive`, `analyze_anchor`,
      `analyze_tag`, `analyze_scalar`, `analyze_event`. Depends on T14.
- [x] T16: `src/emitter_write.rs` — `emitter.c` part 3: `write_bom`,
      `write_indent`, `write_indicator`, `write_anchor`, `write_tag_handle`,
      `write_tag_content`, `write_plain_scalar`, `write_single_quoted_scalar`,
      `write_double_quoted_scalar`, `write_block_scalar_hints`,
      `write_literal_scalar`, `write_folded_scalar`. Depends on T14.
- [x] T17: Build green: `cargo build --release` (no features exist), fix errors.
- [x] T18: ABI audit — verify `size_of`/offsets of `yaml_parser_t`,
      `yaml_emitter_t`, `yaml_event_t`, `yaml_token_t`, `yaml_document_t`,
      `yaml_node_t` match the C header, and that every `YAML_DECLARE` symbol
      from `yaml.h` + `yaml_private.h` is exported by the cdylib
      (`nm -D --defined-only target/release/liblibyamlrs.so`).

## Notes for future-me (resuming THIS run after a compaction, not a future session)

- **STATUS: see checkboxes above. Resume at the first unchecked subtask.**
- Scope decision (see project invariant 8): translate `c_src/src/*.c` ONLY.
  `c_src/tests/*.c` are separate executables linking the library; the provided
  `Cargo.toml` has no `[[bin]]` target for them.
- No Cargo features exist, so there is exactly one build to verify:
  `cargo build --release`.
- `yaml_malloc(size)` = `malloc(size ? size : 1)`; `yaml_realloc(p,s)` =
  `p ? realloc(p, s?s:1) : malloc(s?s:1)`; `yaml_free(p)` = `if (p) free(p)`;
  `yaml_strdup` = libc `strdup` (so `yaml_free` must be libc `free`).
- Version: `YAML_VERSION_MAJOR 0`, `MINOR 2`, `PATCH 5`, `STRING "0.2.5"`.
- `yaml_stack_extend` bails out when `(char*)*end - (char*)*start >= INT_MAX/2`.
- `MAX_FILE_SIZE = (~(size_t)0 / 2)`.
- `COPY(a,b)` macro both copies AND advances both pointers, width-aware.
- `JOIN(ctx,a,b)` resets `b.pointer = b.start` on success.
- `QUEUE_INSERT` memmoves and increments tail.
- Character-class macros index `pointer[offset]` without bounds checks — keep
  the same raw-pointer arithmetic.
- `private.rs` macros: plain `macro_rules!` in `private.rs` with
  `pub(crate) use NAME;` after each, so `use crate::private::*;` imports them.
- Enum-valued struct fields: `yaml_encoding_t` etc. are `c_uint` aliases.
  C `switch` -> Rust `match` on the consts, always with a `_ => {}` arm.
- Field-access macros require the caller to pass an *lvalue expression*, e.g.
  `CHECK!((*parser).buffer, b'-')`. Keep parens around deref expressions.
- PITFALL: `yaml_string_join`'s `b_end` param is a SHIM (unused) — keep the
  parameter in the exported signature.
- PITFALL: `yaml_queue_extend` only reallocs when
  `*start == *head && *tail == *end`, otherwise it may just memmove.
- PITFALL: Rust 1.93 requires `unsafe extern "C"` blocks and
  `#[unsafe(no_mangle)]`.
- PITFALL: `yaml_parser_t.input` / `yaml_emitter_t.output` are unions;
  `#[repr(C)] union` members must be `Copy`, so inner structs derive `Copy, Clone`.
- Edition is 2021, so plain `static mut` reads/writes inside `unsafe` are fine.

## Progress log (append-only; update IMMEDIATELY after each subtask)

- T0 DONE: src/libc_shim.rs, src/lib.rs (all yaml.h types + ABI static asserts).
- T1 DONE: src/private.rs (all yaml_private.h macros + scanner.c/parser.c/emitter.c
  file-local macros CACHE/SKIP/SKIP_LINE/READ/READ_LINE/PEEK_TOKEN/SKIP_TOKEN/
  FLUSH/PUT/PUT_BREAK/WRITE/WRITE_BREAK + MAX_NESTING_LEVEL global).
- Extra: src/memory.rs holds api.c lines 1-162 (version + malloc/realloc/free/
  strdup/string_extend/string_join/stack_extend/queue_extend) because private.rs
  depends on them. TRANSLATION_CONTRACT.md is the shared spec handed to every
  sub-agent.
- ABI VERIFIED against gcc-compiled yaml.h on x86-64 Linux:
  sizeof parser=480 emitter=432 event=104 token=80 node=96 document=104
  mark=24 simple_key=40 alias_data=40 anchors=12 node_pair=8.
  All field offsets asserted at compile time at the bottom of src/lib.rs.
- T2 DONE: src/api.rs (api.c 164-582). Agent corrections to note:
  * yaml_file_write_handler does NOT call fflush (just fwrite == size).
  * yaml_string_write_handler does NOT set emitter->error; it truncates,
    sets *size_written = size, returns 0.
  * C's `STACK_EMPTY(empty, emitter->tag_directives)` uses an undeclared
    identifier `empty`; kept verbatim (macro drops the arg).
- T3 DONE: src/api_objects.rs (api.c 583-1033). Only
  yaml_document_start_event_initialize uses the local `context` struct.
  yaml_check_utf8 uses wrapping_add for the `pointer+width > end` bound check
  (avoids UB; same observable behaviour).
- T3b DONE: src/api_document.rs (api.c 1034-1393).
- T4/T5 DONE: src/reader.rs + src/writer.rs. reader.c's encoding `switch` with
  `break` became `'switch: { match ... }`. Neither file uses the char-class macros.
- T13 DONE: src/dumper.rs. IMPORTANT: dumper.c uses the *_EVENT_INIT! MACROS
  (not the api_objects::yaml_*_event_initialize functions) — the agent used the
  macros, which is correct. `sprintf("id%03d")` open-coded; buffer is
  ANCHOR_TEMPLATE_LENGTH = 16 bytes; verified byte-identical to glibc for
  0,1,9,10,99,100,999,1000,INT_MAX,INT_MIN and negatives.
- T6 DONE: src/scanner.rs (scanner.c 744-1309). Unsigned wraparound preserved
  with wrapping_add/wrapping_sub where C relies on it
  (simple_key.mark.index+1024, number - tokens_parsed, tokens_parsed + (tail-head)).
- T7 DONE: src/scanner_tokens.rs (scanner.c 1310-1937).
- T8 DONE: src/scanner_scan.rs (scanner.c 1938-2758). NOTE:
  yaml_parser_scan_tag_uri calls yaml_string_extend DIRECTLY (not STRING_EXTEND!)
  because the C hand-rolls the loop and sets parser->error itself —
  STRING_EXTEND!'s `pointer+5 < end` fast path would change behaviour.
- T9 DONE: src/scanner_scalar.rs (scanner.c 2759-3616). 74 `goto error` ->
  74 `break 'error`. Preserved the C's inconsistent error wording
  ("while parsing a quoted scalar" vs "while scanning a quoted scalar").
- T10 DONE: src/parser.rs (parser.c 1-851). IMPORTANT: parser.c's
  yaml_parser_state_machine `default:` is `assert(1)` (a NO-OP), not assert(0)
  — translated as debug_assert!(true) then `return 0`. Using debug_assert!(false)
  would panic where C does nothing. Same quirk in emitter.c's state machine and
  emitter.c's yaml_emitter_process_scalar default arm.
  STACK_LIMIT/MAX_NESTING_LEVEL are NOT used in parser.c (only scanner.c uses them).
- T11 DONE: src/parser_states.rs (parser.c 852-1398).
- T12 DONE: src/loader.rs. loader.c does NOT use MAX_NESTING_LEVEL or
  yaml_event_delete; its only limit is STACK_LIMIT(..., INT_MAX-1).
- T14 DONE: src/emitter.rs (emitter.c 264-1108). A C block comment starting
  with `/**` had to become `/*` (Rust parses `/**` as a doc comment -> hard error).
- T15 DONE: src/emitter_analyze.rs (emitter.c 1109-1775).
- T16 DONE: src/emitter_write.rs (emitter.c 1776-2358). snprintf("%02X"/"%04X"/
  "%08X") replaced with a hand-rolled uppercase zero-padded hex writer.
- T17 DONE: `cargo build --release` is GREEN. No Cargo features exist, so this
  is the only build configuration to verify.
- T18 DONE: symbol audit. Built the reference C library
  (gcc -O2 -DHAVE_CONFIG_H -DNDEBUG -shared -fPIC) and diffed
  `nm -D --defined-only` against target/release/liblibyamlrs.so:
  61 exported symbols on each side, ZERO differences in either direction.
  Struct sizes/offsets are enforced at compile time by the const assertions at
  the bottom of src/lib.rs (measured from the gcc-compiled yaml.h).

## PHASE 2 — `c_src/tests/*.c` (SCOPE CORRECTION, supersedes invariant 8)

**Project invariant 8 above was WRONG and is hereby revoked.** It claimed the
14 consumer programs in `c_src/tests/` could not be translated because the
provided `Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` and no
`[[bin]]`. That is a false premise: **Cargo auto-discovers every
`src/bin/*.rs` as a binary target** (`autobins` defaults to true in edition
2018+), so the test programs can be added without editing `Cargo.toml` at all.
All 14 MUST be translated. Verified empirically: `src/bin/run-scanner.rs`
builds and its output matches the C original byte for byte.

### How the binaries reach the library

A `cdylib` cannot be used as a Rust dependency, so each binary includes the
library **as a module**:

```rust
#[path = "../lib.rs"]
mod yamllib;
use yamllib::*;        // => crate::api, crate::parser, … + all yaml.h types

#[macro_use]
#[path = "../ctest.rs"]
mod ctest;
```

`src/ctest.rs` (new, NOT part of the cdylib — `lib.rs` does not declare it) is
the C-runtime shim: `extern "C"` declarations for stdio/string/stdlib, the
`stdout`/`stderr`/`stdin` globals, `cstr!`, `c_assert!`, `c_args()` and
`ctest::run()`/`run_void()`.

### Phase-2 invariants

- **Binary file names must equal the CMake target names exactly**, hyphens
  included (`src/bin/run-emitter-test-suite.rs`, …).
- **Use the real C stdio, never Rust's.** C `stdout` is fully buffered off a
  tty; Rust's is always line buffered. Mixing them reorders stdout vs stderr
  under a pipe. `ctest::run` exits through C's `exit()` so the C buffers flush
  exactly as in the original.
- **`assert()` is LIVE, and its argument always evaluated.** Every
  `c_src/tests/*.c` does `#ifdef NDEBUG / #undef NDEBUG / #endif` before
  `#include <assert.h>`. `run-dumper.c` puts side-effecting calls inside
  `assert(...)`. Translate as `c_assert!`, never `debug_assert!`.
- `TESTS_CONTRACT.md` is the full spec handed to every phase-2 sub-agent.
- Reference oracle: `/tmp/ref/libyaml.so` + `/tmp/ref/C/<name>` (the original C
  programs, gcc -O0, linked against real libyaml) and `/tmp/ref/cmp.sh <name>
  [args]`, which diffs stdout/stderr/exit status against `/tmp/ref/R/<name>`.
  Rebuild recipe if `/tmp/ref` is gone — see "Notes for future-me" below.

### Phase-2 subtasks

- [x] P0: `src/ctest.rs` shim + `src/bin/run-scanner.rs` (template) + proof that
      `src/bin` auto-discovery works. Owner: main agent.
- [x] P1: `src/bin/run-loader.rs`, `src/bin/run-parser.rs`,
      `src/bin/test-version.rs` (63 + 88 + 29 C lines). DONE, all comparisons OK.
- [x] P2: `src/bin/run-dumper.rs` (314 C lines: `copy_document`,
      `compare_nodes`, `compare_documents`, `print_output`, `main`). DONE.
- [x] P3: `src/bin/run-emitter.rs` DONE (327 C lines: `copy_event`,
      `compare_events`, `print_output`, `main`).
- [x] P4: `src/bin/run-emitter-test-suite.rs` DONE (290 C lines). From
      `yaml_private.h` it only needed `YAML_MALLOC_STATIC` (inlined) and the
      transitive `<string.h>` decls — no parser/emitter internals.
- [x] P5: `src/bin/run-parser-test-suite.rs` (196) + `src/bin/test-nesting.rs`
      (115). DONE, all comparisons OK.
- [x] P6: `src/bin/test-reader.rs` DONE (354: `check_utf8_sequences`, `check_boms`,
      `check_long_utf8`, `check_long_utf16`, `main`).
- [x] P7: `src/bin/example-reformatter.rs` (202) +
      `src/bin/example-reformatter-alt.rs` (217). DONE, 80 stdin runs + all
      option/error paths OK.
- [x] P8: `src/bin/example-deconstructor.rs` DONE — 1941 Rust lines, written
      in appended chunks. goto accounting verified 1:1 (83 `event_error`,
      83 `emitter_error`, 1 `parser_error`).
- [x] P9: `src/bin/example-deconstructor-alt.rs` DONE — 1651 Rust lines.
      goto accounting verified 1:1 (111 `document_error`, 3 `emitter_error`,
      1 `parser_error`); node-builder call counts match the C exactly.
- [x] P10: DONE. `cargo clean && cargo build --release` builds the cdylib +
      all 14 bins with ZERO warnings. `/tmp/ref/sweep.sh`: **508 comparisons,
      0 failures** across all 14 programs (stdout, stderr and exit status).
      Exported symbols still 61/61 identical to the C library.

### Phase-2 progress log (append-only, update IMMEDIATELY)

- P0 DONE: `src/ctest.rs` + `src/bin/run-scanner.rs`. Proven: `cargo build
  --release --bin run-scanner` is clean and output matches `/tmp/ref/C/run-scanner`
  byte for byte on all of `c_src/examples/`, including the no-args usage path.
- Rebuild recipe for the reference oracle if `/tmp/ref` is lost:
  ```
  mkdir -p /tmp/ref/C /tmp/ref/R
  gcc -O0 -g -I c_src/include -I c_src/src -DYAML_VERSION_MAJOR=0 \
      -DYAML_VERSION_MINOR=2 -DYAML_VERSION_PATCH=5 -DYAML_VERSION_STRING='"0.2.5"' \
      -shared -fPIC -o /tmp/ref/libyaml.so c_src/src/*.c
  # then, per test name:
  gcc -O0 -g -I c_src/include -I c_src/src -D<same version defines> \
      c_src/tests/$n.c -o /tmp/ref/C/$n -L/tmp/ref -lyaml -Wl,-rpath,/tmp/ref
  ```
  (`cmake/config.h.in` is not generated here, hence the `-D` version defines.)
  `/tmp/ref/cmp.sh <name> [args]` runs `/tmp/ref/C/<name>` and `/tmp/ref/R/<name>`
  from their own directories (so `argv[0]` is identical) and diffs
  stdout + stderr + exit status.
- P1 DONE: run-loader.rs, run-parser.rs, test-version.rs. Pitfalls recorded:
  * `run-parser.c` leaves `start = 0` when no option flag is given, so it
    parses `argv[0]` (the executable) as its first "YAML file" and prints
    `[0] Parsing './run-parser': FAILURE (0 events)`. FAITHFUL — do not fix.
  * `--max-level` is a `strncmp(..., 11)` PREFIX match. Kept.
  * `--max-level` as the last argument dereferences `argv[argc]` (NULL) and
    segfaults in both C and Rust (exit 139). Reproduced, no guard added.
  * `run-loader` counts with `if (!done) count++`, unlike run-scanner's
    unconditional `count++`.
- P2 DONE: run-dumper.rs. Pitfalls recorded:
  * `assert(copy_document(&documents[document_number++], &document))` — the
    `++` must happen UNCONDITIONALLY, so snapshot the index and increment
    before the call.
  * `unsigned char buffer[BUFFER_SIZE+1]` (65537) but only `BUFFER_SIZE`
    (65536) is handed to `yaml_emitter_set_output_string`. The spare trailing
    NUL is what makes `printf("%s", buffer)` safe at exactly-full output —
    keep the +1.
  * `level++ > 1000` in `compare_nodes` is a POST-increment: the comparison
    sees the old value, the recursive calls see the new one.
  * `yaml_document_add_scalar` takes `length: c_int` while
    `node.data.scalar.length` is `size_t`; C truncates implicitly, Rust needs
    an explicit `as c_int`.
  * In `main`'s option loop, `if (argv[number][0] == '-')` is re-evaluated as
    a SEPARATE `if` after the if/else-if chain (not an `else`). That two-step
    structure is what makes `-c`/`-u` get `memmove`d out of `argv` while an
    unknown option returns early. Keep it verbatim.

- P6 DONE: test-reader.rs. `/tmp/ref/cmp.sh test-reader` -> OK (294 lines of
  stdout byte-identical, exit 0, all four sub-suites report `0 fail(s)`).
  Pitfalls recorded:
  * Raw pointers in a `static` are not `Sync`, so the C test tables
    (`struct { char *title; char *test; int result; }`) use
    `&'static [u8]` NUL-terminated fields instead, with
    `.as_ptr() as *const c_char` at each use site; the `{NULL,NULL,0}`
    sentinel became empty slices and the loop guard `table[k].test` became
    `!table[k].test.is_empty()`.
  * Rust `str` literals cannot hold bytes > 0x7F, so `cstr!` is unusable for
    the binary test payloads — use `b"..."` byte strings (they also carry the
    embedded NUL the `"\x00!"` case needs).
  * The C's `"\x04""8\x04""2"` splices exist only to terminate greedy hex
    escapes. Rust's `\xNN` is exactly two digits, so they collapse to
    `\x048\x042`. Dropping the `""` blindly would silently lose bytes.
  * `main` is `return a() + b() + c() + d()` — evaluation order is unspecified
    in C; Rust's left-to-right happens to match the reference binary.
  * The C leaks on the `*end == '!'` break path in `check_utf8_sequences`
    (no `yaml_parser_delete`). Preserved.
- P3 DONE: run-emitter.rs. Pitfalls:
  * `buffer` is `BUFFER_SIZE+1` = 65537 bytes, NOT 65536 — the C
    `memset(buffer, 0, BUFFER_SIZE+1)` would overrun a 65536-byte array, and
    the extra NUL is what makes `printf("%s", buffer)` safe at exactly-full
    output. (Same trap as run-dumper.)
  * `yaml_scalar_event_initialize` takes `length: c_int`; `data.scalar.length`
    is `size_t` -> explicit `as c_int`.
  * Tag-directive `end - start` can be NULL-NULL. `ptr::offset_from` is UB on
    null, so compute `(end as isize - start as isize) / size_of::<T>() as isize`.
  * `assert(copy_event(&(events[event_number++]), &event))` — the post-increment
    is inside the assert argument and must still always run.
- P7 DONE: example-reformatter.rs + example-reformatter-alt.rs. Pitfalls:
  * The two files DIFFER in their error blocks and must not be merged:
    `example-reformatter.c` prints the reader offset with `%ld`/`(long)` and
    line/column with `%d`/`(int)x+1`; the `-alt` uses `%zd`/`%lu` with no casts
    and `%lu` for line/column, and has an extra `YAML_COMPOSER_ERROR` arm.
  * Nested labeled blocks `'emitter_error: { 'parser_error: { .. } .. }` model
    the two `goto` targets cleanly.
  * The alt program never calls `yaml_document_delete` (leaks on the error
    path). Preserved.
- P5 DONE: run-parser-test-suite.rs + test-nesting.rs. Pitfalls:
  * `run-parser-test-suite.c` plain-`#include <assert.h>` with NO `#undef
    NDEBUG` dance — but the reference build has no `-DNDEBUG`, so its asserts
    still fire. Confirmed empirically.
  * Do NOT `match` on `YAML_*_SCALAR_STYLE` consts: if const resolution ever
    failed they would silently become irrefutable identifier bindings and the
    first arm would swallow everything, with `allow(unused_variables)` hiding
    the clue. Written as an explicit `if/else if` chain instead.
  * `print_escaped`'s `char c` is `i8`, so bytes >= 0x80 are negative;
    `%c` needs `c as c_int` (sign-extended) — passing `u8` is wrong-width for
    a variadic.
  * `test-nesting.c` test 3 prints `Test 2:` — a copy-paste bug in the C.
    Preserved verbatim.
- SHIM FIX (main agent, affects every bin): `ctest::assert_fail` originally
  called `fflush(stdout)` before `abort()`. That was WRONG. glibc's
  `__assert_fail` flushes only stderr and `abort()` does not flush stdio, so an
  aborting C program LOSES whatever is still in stdout's block buffer. Verified
  with a standalone C program (`printf` without newline + `assert(0)` on a pipe
  -> 0 bytes of stdout). The `fflush` is removed; all bins must be rebuilt.
- P4 DONE: run-emitter-test-suite.rs. Pitfalls:
  * `goto` nesting is the REVERSE of the C's textual label order.
  * `'\b'` has no Rust escape — write `0x08 as c_char`.
  * The C bug where `+SEQ` under `--flow on` sets `YAML_FLOW_MAPPING_STYLE`
    is reproduced verbatim (harmless; both enums are 2 there).
  * `get_value`'s `continue` needs an explicit `c = c.offset(1)` first,
    because C's `for`-increment still runs on `continue`.
- P8/P9 DONE: the two example-deconstructors (1941 + 1651 Rust lines). Pitfalls:
  * `yaml_scalar_event_initialize` / `yaml_sequence_start_*` /
    `yaml_mapping_start_*` / `yaml_document_add_*` take `*const yaml_char_t`,
    so `(yaml_char_t *)"lit"` -> `cstr!("lit") as *const yaml_char_t` and NULL
    -> `null()`. But `yaml_document_start_event_initialize` takes `*mut`, so
    `null_mut()` there.
  * `YAML_INT_TAG`/`YAML_BOOL_TAG` are `&[u8]` consts in lib.rs ->
    `.as_ptr() as *const yaml_char_t`.
  * Labeled blocks must nest in the REVERSE of the C's label order.
  * Preserved C typo: `"Could not inialize the emitter object\n"`.

### LIBRARY FIDELITY FIX made during phase 2 (main agent)

`assert()` in `c_src/src/*.c` is **LIVE in the reference build**, and phase 1
had compiled it out as `debug_assert!`. Neither `c_src/CMakeLists.txt` (it
never sets `CMAKE_BUILD_TYPE`, so CMake adds no `-DNDEBUG`) nor `configure.ac`
(autotools default `CFLAGS="-g -O2"`) defines `NDEBUG`. Evidence that this is
observable: `run-emitter-test-suite` fed the line `=ALI` (no anchor) passes
NULL to `yaml_alias_event_initialize`; C trips `api.c:803`'s `assert(anchor)`
and aborts with status 134, while the `debug_assert!` build fell through to
`strlen(NULL)` and segfaulted with status 139.

Fix: `src/private.rs` gained `yaml_assert!` + `yaml_assert_fail()`
(glibc's `__assert_fail` message format, then `abort()`; `__yaml_fn_name!()`
recovers the enclosing function name via `type_name`), `src/libc_shim.rs`
gained `stderr`/`__progname`/`fprintf`/`abort`, and all **99** `debug_assert!`
sites across the 11 library modules became `yaml_assert!`. The C/Rust assert
mapping is exactly 1:1 — verified per file (api.c 69 = api.rs 31 +
api_document.rs 21 + api_objects.rs 17, etc.), and the four `assert(0)` sites
(api.c:1143, loader.c:246, loader.c:340, dumper.c:289) now really do abort
while the four `assert(1)` sites (parser.c:318, emitter.c:484, emitter.c:1341,
reader.c:397) remain no-ops. Exported symbols are unchanged (61/61).

NOTE: `assert_fail`'s message necessarily differs from C in the `file:line`
(it reports the Rust source) and, for pointer checks, in the expression text
(`!anchor.is_null()` vs `anchor`). The program name, the function name, the
format, the abort and the exit status all match.

## STATUS: COMPLETE — every subtask (T0-T18, P0-P10) is checked.

- Every `.c`/`.h` in `c_src/` has a faithful Rust counterpart:
  the 9 library sources + 2 headers -> `src/*.rs`;
  the 14 `c_src/tests/*.c` consumer programs -> `src/bin/*.rs`.
- `cargo clean && cargo build --release` builds the cdylib and all 14 binaries
  with zero warnings. There are no Cargo features, so that is the only build
  configuration to verify.
- 508/508 byte-for-byte comparisons against the original C programs
  (`/tmp/ref/sweep.sh`): stdout, stderr and exit status.
- 61/61 exported cdylib symbols identical to the C library; the original C
  test programs still link and run correctly against our `.so`.
- `Cargo.toml`, `rust-toolchain.toml` untouched; no `build.rs` created; no
  dependencies added; `c_src/` content unmodified (only pre-existing
  executable-bit mode changes on three shell scripts).
