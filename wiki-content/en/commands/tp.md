## Description

`/tp` teleports you instantly to precise coordinates. Ideal for exploring, finding a build again, or taking a close look at a mountain peak spotted in the distance.

## Syntax

`/tp <x> <y> <z>`. Each coordinate can be:
- a number (decimals allowed);
- `~`: your own current coordinate;
- `~` followed by a number: an offset from your position (`~5`, `~-10`).

## Examples

- `/tp 0 200 0`: off to the centre of the world, well above the ground.
- `/tp ~ ~20 ~`: rise twenty blocks on the spot.
- `/tp ~100 ~ ~`: a quick hop to the east.

## Behaviour

Run by the server, in **creative mode** only. You arrive **flying** and still: no surprise fall. The destination is brought back inside the world border and its height. The console confirms the coordinates actually reached.

## Tips

Write down the coordinates shown by F3 before setting off: you'll be able to come back with a single `/tp`.

## Discovery in game

`/help tp` recalls the syntax, and the Tab key completes the command name. F3 shows your position so you can work out destinations.
