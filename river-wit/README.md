# river-wit

WIT → S-expression bindings, a host capability bridge, and introspection.

This is **not** `wit-bindgen` and it does not emit guest code. `wit-bindgen`
generates guest bindings for supported languages (Rust, C, C++, C#, Go, …).
River needs a different layer: parse WIT with `wit-parser`, emit readable
S-expressions, build a capability catalog, and let a host-side Lisp/interpreter
call the capabilities behind a grant check.

## Generate

```sh
cargo run -p river-wit -- ../../soil/wit/soil-records.wit --out generated/soil-records.river
```

Output:

```lisp
(define-package "soil:records@0.1.0" :base "soil:records" :version "0.1.0")

(define-record soil:records/entry
  (key soil:records/key)
  (schema string)
  ...
)

(define-function soil:records/scan
  ((collection string) (cursor (option string)) (limit u32))
  -> (result soil:records/page soil:records/error)
  :grant "soil:records" :version "0.1.0")
```

Generated files in [`generated/`](generated) cover the first capability set:
records, unibus, time, principal, actions, and BLE.

## Host bridge

```rust
let generated = river_wit::generate("../../soil/wit/soil-records.wit")?;
let result = generated.catalog.call(
    &mut host,                         // Host trait
    "soil:records/scan",
    &[Value::String("signal:avery".into()), Value::Nil, Value::Int(50)],
)?;
```

`Catalog::call` checks:

1. the capability exists,
2. the host grants it,
3. arity matches,
4. basic argument types match,

then calls the host. The host owns the real side effect and the actual grant
policy; the catalog is a projection, not an authority.

A Lisp-shaped form goes through the same path:

```lisp
(soil:records/scan "signal:avery" nil 50)
```

```rust
river_wit::call_form(&catalog, &mut host, &form)?;
```

## Introspection

`river_wit::eval_form` answers questions about the generated surface as data:

```lisp
(capabilities)
(grants)
(interfaces)
(types)
(describe "soil:records/scan")
```

`describe` returns a tagged list with `:signature`, `:grant`, `:version`,
`:params`, and `:result`. That is the same S-expression world the agent can
feed back into the next decision.

## What this does and does not unlock

**Today:** host-side Lisp capability scripts. The generated symbols are
ordinary forms; the Rust host executes them through the bridge. This is enough
to write River/Instinct logic against real WIT contracts.

**Not yet:** a signed Wasm Spore whose engine is Lisp. That needs a host profile
that embeds a Lisp runtime (`[engine].target = "lisp"` or similar), or a
guest-side Compenent Model backend with canonical-ABI lifting/lowering. This
crate deliberately stops at the host boundary.

## Limits

- Resources, handles, futures, streams, maps, and fixed-length lists are
  emitted as `(define-unsupported …)`; they need lifetime/ownership design.
- Argument validation is basic (arity + primitives/lists/options/results).
  Named-type validation needs catalog-aware checking.
- One package per invocation. Multi-package dependency graphs are next.
- The generator is not a substitute for `wit-bindgen` on guest targets.

## Test

```sh
cargo test -p river-wit
```
