## Description

`/fluids` lets the world's water and lava flow or freezes them, and sets how many liquid blocks the server updates each tick. The setting is saved with the world; it can also be chosen when creating the world (the "Liquids" button).

## Syntax

`/fluids [on|off|query|<budget>]`
- `on`: liquids flow.
- `off`: liquids stay as they are, where they are.
- `query` (or nothing): shows the current setting.
- `<budget>`: liquid updates per tick at most (2048 by default); the rest waits for the next tick.

## Examples

- `/fluids off`: freezes a lake while you build.
- `/fluids 512`: slower but cheaper flows, handy on a small machine.

## Behaviour

Run by the server, in **creative mode** only. Freezing liquids forgets the blocks that were flowing: they only start again when a block beside them changes after `/fluids on`.

## Tips

A smaller budget does not change the final shape of a flow, only its speed when a lot of water moves at once.

## Discovery in game

`/help fluids` recalls the syntax; Tab completes `on`, `off` and `query`.
