## Description

`/daycycle` lets the world's time of day go on or stops it. The setting is saved with the world; it can also be chosen when creating the world (the "Day and night" button).

## Syntax

`/daycycle [on|off|query]`
- `on`: time goes on, the sun and moon move.
- `off`: time stays where it is (handy with `/time set` to always build in broad daylight).
- `query` (or nothing): shows the current setting.

## Examples

- `/time set noon` then `/daycycle off`: noon forever.
- `/daycycle on`: the day goes on again.

## Behaviour

Run by the server, in **creative mode** only. Time also stands still while the game is paused.

## Tips

`/time add` works even with the time stopped: move on a few hours without starting the cycle again.

## Discovery in game

`/help daycycle` recalls the syntax; Tab completes `on`, `off` and `query`.
