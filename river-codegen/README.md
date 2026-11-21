# river-codegen

The first River → WebAssembly backend. It compiles the **frozen numeric core**:

- `defun`, `let`, `if`, `cond`
- `+ - * /`
- `= > < >= <=`
- `and`, `or`, `not`
- calls between numeric functions

Every value is `i64`; every function is `(i64, ...) -> i64`. Booleans are `1`/`0`.

The core deliberately rejects lists, strings, keywords, records, JSON, and
`host-call`. Those need a value representation and a WIT import plan first; this
backend is the differential-tested seed for that work.

```sh
cargo run -p river-codegen --bin riverc -- program.river program.wasm
```

Tests instantiate the module with wasmtime and compare results against
`river-run` on the same numeric programs.

## Next

- Add `bool` as a real type and short-circuit `and`/`or`.
- Add strings and a small linear-memory representation.
- Lower `soil:records` host calls to WIT imports.
- Differential-test compiled Instinct rules against the `.river` interpreter.
