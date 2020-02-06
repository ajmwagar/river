
<div align="center">
 <p><h1>River 🌊</h1> </p>
  <p><strong>A scripting language for data streams.</strong></p>
<p>
  
[![GitHub Actions](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Factions-badge.atrox.dev%2Fajmwagar%2Fart%2Fbadge&label=build&logo=none)](https://actions-badge.atrox.dev/ajmwagar/art/goto)
[![crates.io](https://meritbadge.herokuapp.com/[CRATE_NAME])](https://crates.io/crates/[CRATE_NAME])
[![Documentation](https://docs.rs/[CRATE_NAME]/badge.svg)](https://docs.rs/[CRATE_NAME])
[![Download](https://img.shields.io/crates/d/[CRATE_NAME].svg)](https://crates.io/crates/[CRATE_NAME])
[![Version](https://img.shields.io/badge/rustc-1.40+-lightgray.svg)](https://blog.rust-lang.org/2019/12/19/Rust-1.40.0.html)
[![License](https://img.shields.io/crates/l/[CRATE_NAME].svg)](./LICENSE)

  </p>
</div>
<br>

## Features
- `async` by default
- Built for data streams
- Easy encoding/decoding between formats
- Easy async command line access

## Examples

#### Hello, World!

```river
#!/bin/river
use io

"Hello, World!" > io.stdout
```

#### Data encoding/decoding (CSV to JSON)
```
#!/bin/river
use io, encode, decode

io.stdin > decode.csv > encode.json > io.stdout
```

