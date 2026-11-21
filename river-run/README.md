# river-run

A small closed evaluator and CLI for River S-expressions.

It is deliberately not a general Lisp. It evaluates:

- `defun`, `let`, `if`, `cond`, `and`/`or`/`not`, `begin`
- arithmetic and comparisons
- `list`, `field`/`get`, `length`, `str`, `println`, `json-decode`, `json-encode`
- `ok?`, `err?`, `value`, `nil?`, `first`, `rest`, `cons`, `append`
- `apply`, `string-contains?`
- exactly one host boundary: `(host-call interface function args)`

The generated WIT bindings define `defun` stubs over `host-call`. `river-run`
does not know what a device, signal, or profile is; those are `.river` programs.

`FakeHost` is a generic in-memory records host for tests. `SoilRecordsHost`
links `fungos-records` directly, in-process, for real runs. `SystemTimeHost`
serves `soil:time`.

## Run

```sh
TMP=$(mktemp -d)
cargo run -p river-run -- \
  --prelude ../prelude.river \
  --records-root "$TMP" --book river \
  ../river-wit/generated/soil-records.river \
  ../river-wit/generated/soil-time.river \
  examples/seed_instinct.river \
  examples/instinct.river
```

`seed_instinct.river` writes the fixture through `soil:records/upsert`.
`instinct.river` scans it, classifies each device, and fuses person presence:

```text
((:devices (("az09:1" (:in-use 0.85)) ...)) (:person (:present 0.9)))
```

Options:

```text
--call NAME          call this function after loading (default: main)
--no-call            only load and evaluate top-level forms
--prelude FILE       load a River prelude before the program files
--records-root DIR   use the real in-process soil:records host at DIR
--book NAME          fungos-records book to open (default: river)
```

## Host bridge

```lisp
(soil:records/scan "signal:avery" nil 50)
;; generated stub:
(host-call "soil:records" "scan" (list "signal:avery" nil 50))
```

For a narrow grant in tests:

```rust
FakeHost::new().grant("soil:records", "scan")
```

or deny everything:

```rust
FakeHost::new().deny_all()
```

For a real run, `--records-root` opens `fungos-records` in-process. There is no
HTTP hop: `host-call` maps to the same store the Soil records host uses.
`POST /v1/call` remains only for out-of-process clients (browser pages, external
MCP).

## What is next

- Package the River prelude and Instinct program as a Spore with a `river`
  engine target.
- Freeze the typed core and add a Wasm backend.
