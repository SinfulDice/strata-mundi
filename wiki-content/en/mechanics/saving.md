## Principle

Each world has its own folder: its description (name, seed, mode, play time), your character (position, view direction, hotbar) and a screenshot for the world list. Since the terrain can be recalculated from the seed, **only changed areas** are saved: a well-explored but barely altered world stays tiny.

## When the game saves

- **Automatically**, at regular intervals (adjustable in Options → Game).
- With **Save and quit**, in the pause menu.
- When you close the game window.
- On the DS, when you **close the lid**: instant pause and save.

## Never a damaged world

Each file is written alongside, then swapped in at once: a power cut leaves either the old or the new version, never a mix. An unreadable file is set aside, not overwritten.

## Backups

Each time a world is opened, a full copy is made; the most recent ones are kept (how many is set in Options → Game). Deleting a world keeps its backups.

## Versions

A world from an incompatible version appears greyed out in the list, with the reason.

## Discovery in game

The world list shows screenshot, date, mode and play time; the delete confirmation reminds you that backups are kept.
