# river-wit

WIT → S-expression bindings.

The generated output is deliberately small:

- type definitions,
- capability metadata for introspection,
- one `defun` stub per WIT function that calls a single generic host function.

There is **no runtime catalog**, **no per-capability host adapter**, and **no
`wit-bindgen` dependency**. `wit-parser` reads the WIT at generation time; the
host owns grants and dispatch at runtime.

## Why not `wit-bindgen`

`wit-bindgen` generates guest bindings for Rust, C, C++, C#, Go, and so on. It
has no Lisp backend, and River needs a different boundary: generated Lisp stubs
that call the Soil host through the existing generic call surface. `wit-parser`
is the reusable part.

## Generate

```sh
cargo run -p river-wit -- ../../soil/wit/soil-records.wit --out generated/soil-records.river
```

Output:

```lisp
(define-record soil:records/entry
  (key soil:records/key)
  (schema string)
  (revision u64)
  ...
)

(define-capability soil:records/scan
  :interface "soil:records"
  :function "scan"
  :grant "soil:records"
  :version "0.1.0"
  :params ((collection string) (cursor (option string)) (limit u32))
  :result (result soil:records/page soil:records/error))

(defun soil:records/scan (collection cursor limit)
  (host-call "soil:records" "scan" (list collection cursor limit)))
```

`define-capability` is introspection data. `defun` is the callable stub.

## The host call

The Lisp runtime provides exactly one function:

```lisp
(host-call interface function args)
```

It maps to the Soil host surface:

```http
POST /v1/call
{"interface":"soil:records","function":"scan","params":{...}}
```

or, in a host-embedded Lisp, to the same `__soil.invoke` boundary the Lua SDK
uses. The host checks the Spore manifest's grants before dispatch. The generated
Lisp contains no authority.

A script can therefore do:

```lisp
(soil:records/scan "signal:avery" nil 50)
;; =>
(host-call "soil:records" "scan" (list "signal:avery" nil 50))
```

## Introspection

Because `define-capability` is data, the Lisp side can answer:

```lisp
(capabilities)                 ; names
(grants)                       ; grants referenced by the generated forms
(describe "soil:records/scan") ; the whole capability form
```

Those are ordinary list operations over the generated document. No Rust catalog
is involved.

## What this unlocks

**Host-side Lisp capability scripts: yes.** Generate the bindings, load them in
the Lisp environment, and call the host through the bridge. The host is the
runtime; the script is the program; WIT is the API.

**Signed Wasm Spores with a Lisp engine: not yet.** That needs a host profile
that embeds Lisp (`[engine].target = "lisp"`) or a guest-side Component Model
backend. This crate deliberately stops at the host call.

## Limits

- Resources, handles, futures, streams, maps, and fixed-length lists are emitted
  as `(define-unsupported …)`.
- Type definitions and capability metadata are emitted; full schema validation
  and grant-scoped generation are next.
- One package per invocation. Multi-package dependency graphs are next.
- `host-call` is a contract, not an implementation. The runtime provides it.

## Test

```sh
cargo test -p river-wit
```
