## Description

`/give` adds items to your inventory. Faster than digging through the creative inventory when you already know the identifier of what you want.

## Syntax

`/give <item> [count]`, where the item is an identifier:
- full, with its namespace: `strata:stone`;
- or short, for the base content: `stone` means `strata:stone`.

The count is 1 when left out.

## Examples

- `/give glass` for a brand new pane.
- `/give strata:fir_log 128` for two stacks of fir logs.

## Behaviour

Run by the server, in worlds where cheating commands are allowed. Items join the existing stacks, then the free slots (hotbar first); what does not fit falls to the ground in front of you. An unknown identifier is reported and nothing changes. Some blocks have no item (water, for instance) and cannot be given.

## Tips

Press Tab while typing: the console completes item identifiers. Handy to find a block's exact name.

## Discovery in game

`/help give` recalls the syntax. To learn a block's identifier, aim at it with F3 on: the "aimed block" line shows it.
