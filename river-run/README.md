# river-run

A small closed evaluator and CLI for River S-expressions.

It is deliberately not a general Lisp. It evaluates:

- `defun`, `let`, `if`, `cond`, `and`/`or`/`not`, `begin`
- arithmetic and comparisons
- `list`, `field`/`get`, `length`, `str`, `println`
- `ok?`, `err?`, `value`, `nil?`, `first`, `rest`
- exactly one host boundary: `(host-call interface function args)`

The generated WIT bindings define `defun` stubs over `host-call`. The host owns
grants and dispatch. `FakeHost` is an in-memory records host for the first slice.

## Run

```sh
cargo run -p river-run -- \
  ../river-wit/generated/soil-records.river \
  examples/records.river
```

`examples/records.river` scans `signal:avery` through the generated stub and
prints the record count.

```text
records: 2
```

Options:

```text
--call NAME   call this function after loading (default: main)
--no-call     only load and evaluate top-level forms
```

## Host bridge

```lisp
(soil:records/scan "signal:avery" nil 50)
;; generated stub:
(host-call "soil:records" "scan" (list "signal:avery" nil 50))
```

`FakeHost` grants all `soil:records` calls by default. For a narrow grant:

```rust
FakeHost::new().grant("soil:records", "scan")
```

or deny everything:

```rust
FakeHost::new().deny_all()
```

The real host will implement the same `Host` trait over `/v1/call` or
`__soil.invoke`.

## What is next

- Replace `FakeHost` with the web bridge host (`POST /v1/call`).
- Run the Instinct presence profiles as `main.river`.
- Then package a `.river` program as a Spore with a `river` engine target.
