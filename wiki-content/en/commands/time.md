## Description

`/time` reads or sets the world clock. Each world keeps its own time, which moves forward on its own during play (except while paused or with `/daycycle off`) and is saved with the world. The sky follows it: sun, moon, stars and daylight.

## Syntax

`/time <set|add|query> [value]`
- `query` (or nothing): shows the current time and the day number.
- `set <value>`: moves on to the next moment asked for, with a keyword (`day` just after sunrise, `noon` for midday, `night` just after sunset, `midnight`) or a number of ticks after sunrise.
- `add <ticks>`: moves the clock forward by that many ticks.

## Examples

- `/time set noon`: high noon.
- `/time set night`: night falls, the moon rises.
- `/time add 24000`: one day later, at the same time.

## Behaviour

Run by the server, in **creative mode** only. A day on Terra lasts 24,000 ticks (20 minutes): half of it day, half night. Time never goes back: `set` moves on to the next moment asked for, the next day if it has already passed. The moon changes phase every night, over a cycle of eight nights.

## Tips

The phases of the moon count the days: eight nights pass from one full moon to the next.

## Discovery in game

`/help time` recalls the syntax and keywords; Tab completes `set`, `add`, `query` and the times of day.
