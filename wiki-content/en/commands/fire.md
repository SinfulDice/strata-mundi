## Description

`/fire` sets how fire behaves in this world. The setting is saved with the world; it can also be chosen when creating it (the "Fire" and "Burning" buttons).

## Syntax

`/fire spread on|off`, `/fire destroy on|off`, `/fire query`
- `spread off`: fire no longer lights new fires around it; it wears out and dies where it is.
- `destroy off`: what burns is no longer destroyed (fire still dies out in the end).
- `query` (or nothing): shows both settings.

## Examples

- `/fire spread off`: a fireplace that never sets the house alight.
- `/fire destroy off` then `/fire spread on`: a spectacular blaze that leaves no trace.

## Behaviour

Run by the server, in **creative mode** only. Fires already burning follow the new setting at once.

## Discovering it in game

`/help fire` recalls the syntax; Tab completes `spread`, `destroy`, `query`, then `on` and `off`.
