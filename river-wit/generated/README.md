# Generated WIT bindings

Regenerate from the canonical Soil WITs:

```sh
cargo run -p river-wit -- ../../soil/wit/soil-records.wit --out generated/soil-records.river
cargo run -p river-wit -- ../../soil/wit/soil-unibus.wit --out generated/soil-unibus.river
cargo run -p river-wit -- ../../soil/wit/soil-time.wit --out generated/soil-time.river
cargo run -p river-wit -- ../../soil/wit/soil-principal.wit --out generated/soil-principal.river
cargo run -p river-wit -- ../../soil/wit/fungos-actions.wit --out generated/fungos-actions.river
cargo run -p river-wit -- ../../soil/wit/soil-ble.wit --out generated/soil-ble.river
```

These files are the readable surface. The machine-side catalog is built from
the same WIT in `river_wit::generate`; the `.river` files exist so the agent can
read, diff, and eventually load them into a Lisp environment.
