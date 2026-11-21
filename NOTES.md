# River in 2026

Name?
Tide?
Creek?
River?

https://github.com/ajmwagar/river

Goals:
- Compile to WASM "Spores"
- Hardware Accel Everything
- GPU Resident Everything
- Craft amazing experiences.
- Dynamic Topology/Reconfiguration?

Abstract away complex system / routing.

Concept for experiences.


Primitives:

Frame (Image?) Picture or Video encoding is handled.
Audio?

### Usecases
- Video to NVR to alerts
- Audio to transcribe + llm summary
- Click Omi = Superwhisper.

- HW Events to actions? (Dispatch?)
- DJ Set the mood? No? hmm.


## Agent edits, live (instant.nvim + MCP)

- instant.nvim syncs Neovim <-> Neovim.
- `mcp/server.mjs` lets an MCP agent drive this Neovim over its RPC socket.
- Topology: agent -> MCP -> local Neovim -> instant.nvim -> every other client.
- Try `:InstantStartSingle`, then point the MCP server at this nvim socket.
