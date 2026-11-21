# river-run

A small closed evaluator and CLI for River S-expressions.

It is deliberately not a general Lisp. It evaluates:

- `defun`, `let`, `if`, `cond`, `and`/`or`/`not`, `begin`
- arithmetic and comparisons
- `list`, `field`/`get`, `length`, `str`, `println`, `json-decode`, `json-encode`
- `ok?`, `err?`, `value`, `nil?`, `first`, `rest`
- exactly one host boundary: `(host-call interface function args)`

The generated WIT bindings define `defun` stubs over `host-call`. The host owns
grants and dispatch. `FakeHost` is an in-memory records host for tests;
`SoilRecordsHost` links `fungos-records` directly, in-process, for real runs.

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
--records-root DIR  use the real in-process soil:records host at DIR
--seed              seed the Instinct device/signal fixture (with --records-root)
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

For a real run, `--records-root` opens `fungos-records` in-process. There is no
HTTP hop: `host-call` maps to the same store the Soil records host uses.

```sh
cargo run -p river-run -- \
  --records-root /tmp/river-records --seed \
  ../river-wit/generated/soil-records.river \
  examples/instinct.river
```

`POST /v1/call` remains only for out-of-process clients (browser pages, external
MCP); it is not on the Rust runtime path.

## What is next

- Run the full Instinct presence profiles as `main.river` (the first classifier slice is in place).
- Then package a `.river` program as a Spore with a `river` engine target.
