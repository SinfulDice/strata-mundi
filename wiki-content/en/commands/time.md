## Description

`/time` reads or sets the world clock. Each world keeps its own time, which moves forward on its own during play and is saved with the world.

## Syntax

`/time <set|add|query> [value]`
- `query`: shows the current time.
- `set <value>`: sets the time, with a keyword (`day` for sunrise, `noon` for midday, `night` for sunset, `midnight`) or a number of ticks.
- `add <ticks>`: moves the clock forward by that many ticks.

## Examples

- `/time set noon`: high noon.
- `/time query`: what time is it?

## Behaviour

Run by the server, in **creative mode** only. The answer gives the time in ticks, out of the length of a full day; past that, the clock simply starts again from zero. In v0.1, time is counted and saved, but **the sky doesn't change yet**: the visible day-night cycle will come later.

## Tips

The keywords are a quarter of a day apart: handy to find your way without doing sums.

## Discovery in game

`/help time` recalls the syntax and keywords; Tab completes `set`, `add`, `query` and the times of day.
