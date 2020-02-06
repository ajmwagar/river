
<div align="center">
 <p><h1>River 🌊</h1> </p>
  <p><strong>A scripting language for data streams.</strong></p>
<p>
  
 **WIP** This language is theroretical at this point. I hope it'll become a reality soon.
  
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

#### TCP/UDP sockets

```
#!/bin/river
use io, encode, decode, net

# Connect to a TCP server at 0.0.0.0:3000
("0.0.0.0", 3000) > net.tcp.connect >= socket

# Convert stdin to bson and send over socket
io.stdin > decode.json > encode.bson > socket

# Read response from socket, convert to JSON, and print to stdout
socket > decode.bson > encode.json > io.stdout
```
> TCP Client

```
#!/bin/river
use io, encode, decode, net

# Bind a TCP server to port 3000
("0.0.0.0", 3000) > net.tcp.bind >= server

# Create a handler to:
# 1. Log the peer's IP to stdout
# 2. Echo the stream back to the client
handler(peer_sock) > {
  peer_sock.addr > io.stdout
  peer_sock > peer_sock
}

# Pass the incoming connections (asyncronously) to our handler
server | handler

"Binded to " + server.local_addr > io.stdout
```
> TCP Server
