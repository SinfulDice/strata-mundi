## Description

`/give` drops an item straight into the selected slot of your hotbar. Quicker than rummaging through the creative inventory when you already know the identifier of what you want.

## Syntax

`/give <item>`, where the item is an identifier:
- full, with its namespace: `strata:stone`;
- or short, for the base content: `stone` is the same as `strata:stone`.

## Examples

- `/give glass` for a brand-new pane.
- `/give strata:fir_log` for a fir log.

## Behaviour

Run by the server, in **creative mode** only. The item **replaces** whatever was in the selected slot. An unknown identifier is reported, and nothing changes. Some blocks have no item (water, for example) and so can't be given.

## Tips

Press Tab while typing: the console completes item identifiers. Handy for discovering a block's exact name.

## Discovery in game

`/help give` recalls the syntax. To learn a block's identifier, aim at it with F3 on: the "Targeted block" line shows it.
