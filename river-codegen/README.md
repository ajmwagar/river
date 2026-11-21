# river-codegen

River's first WebAssembly backends.

## Numeric core

`core.rs` + `wasm.rs` compile:

- `defun`, `let`, `if`, `cond`
- `+ - * /`
- `= > < >= <=`
- `and`, `or`, `not`
- calls between numeric functions

Every value is `i64`; booleans are `1/0`.

```sh
cargo run -p river-codegen --bin riverc -- program.river program.wasm
```

## Lisp core

`lisp.rs` compiles the McCarthy-style Lisp kernel: symbols, cons cells,
`quote`, `atom`, `eq`, `car`, `cdr`, `cons`, `cond`, recursion, and calls.

Values are tagged `i64`:

```text
nil                0
symbol(id)         01 | id
cons(address)      10 | address
```

Cons cells live in linear memory; quoted data is laid out statically. This
backend runs the `eval.` from McCarthy's 1960 paper (via Paul Graham's CL
translation) on Wasm.

```sh
cat > /tmp/jmc.river <<'EOF'
(defun caar (x) (car (car x)))
(defun cadr (x) (car (cdr x)))
(defun cdar (x) (cdr (car x)))
(defun caddr (x) (car (cdr (cdr x))))
(defun cadar (x) (car (cdr (car x))))
(defun caddar (x) (car (cdr (cdr (car x)))))
EOF
cat ../river-run/tests/fixtures/jmc.lisp >> /tmp/jmc.river
cargo run -p river-codegen --bin riverc -- --lisp /tmp/jmc.river jmc.wasm
```

Tests instantiate the module with wasmtime and run `eval.` on quoted forms.

## Next

- Add a real `bool` type and short-circuiting to the numeric core.
- Add symbols/lists to River itself, not just the Lisp bootstrap.
- Lower `soil:records` / `soil:time` host calls to WIT imports.
- Differential-test compiled Instinct rules against the `.river` interpreter.
