## Principle

**Chests** and **barrels** come in four tiers: wood (27 slots), iron (45), gold (63) and diamond (81). From iron on, each tier adds **upgrade slots** (1 to 3), shown in the panel to the right of the screen. When a storage has more than five rows, its grid scrolls (buttons of the right panel, mouse wheel).

## Going up a tier without emptying

A **storage kit** used (right click) on a chest or a barrel makes it the next tier **in place**: contents and upgrades are kept. A double chest raises both halves at once: hold **two kits**, so it keeps a single tier. The next tier can also be crafted around a chest (eight ingots), but it must then be emptied.

## Upgrades

- **Auto-sorting**: the storage sorts itself by type when its screen closes, or when it takes items in on its own.
- **Filter**: nine cells to set with a held item; in "allows the list" mode only those items come in by shift click, quick deposit and magnet; in "refuses the list" mode, everything but them. An empty filter lets everything through. By hand, anything can still be put in.
- **Compression**: nine nuggets become an ingot, nine ingots a block, as soon as there is room.
- **Destruction**: what comes in by itself (shift click, quick deposit, magnet) and its filter selects is destroyed; what was already stored or put by hand stays. An empty filter destroys nothing.
- **Magnet**: draws the items lying within 5 blocks, twice a second (world option "Magnets" turns it off).

An upgrade goes only once in a storage; in a double chest, those of both halves work for the whole chest. Automation upgrades (hopper, conveyor) will come with the machines.

## Backpacks

A **backpack** is worn on the back (the « Back » slot of the inventory, beside the off hand) or kept in the inventory. It opens with its key (B by default, d-pad up on a gamepad) when worn, or with a right click in hand. Sneak + right click **puts it down** like a block; broken, it becomes the backpack again with **all it holds**, never scattered, and it does not burn.

Four tiers: leather (27 slots, 1 upgrade slot), iron (45, 2), gold (63, 3), diamond (81, 4). A **backpack kit** raises a backpack **put down** by one tier, without emptying it. No recipe uses up a backpack: its contents are never lost.

Its upgrades are the chests' (sorting, filter, compression, destruction, magnet: it then draws items to you) and its own:

- **Pickup**: what you pick up goes into the backpack first, if its filter lets it in (and its destruction applies).
- **Feeding**: will feed you from what it holds when you are hungry (hunger comes with the survival gauges).
- **Built-in crafting**: a 3×3 grid in the backpack's screen.
- **Tank**: holds up to 8 buckets of one liquid; with a full bucket in hand, the « Held bucket » button pours it in, with an empty one it fills it.

**No bag in a bag**: no backpack or pouch goes into a bag, by hand, by shift click or on pickup.

## Pouches and satchels

Small (9 slots, no upgrades), they **fill up by themselves on pickup** with items of their kind, before the inventory: seed pouch (flowers and mushrooms, then seeds), ore satchel (ores, coal, diamonds, nuggets), ammunition pouch and potion satchel (for the ammunition and potions to come). A right click in hand opens them.

## For pack makers

Folders `storage_tiers/` (`StorageTier(rows, upgrade_slots, upgrade_family)`) and `compression/` (`Compression(from, to, count: 9)`); block field `container: Some(Container(joins, upgradable: Some(Upgradable(family, tier))))`; kit `kit: Some(Kit(on: Storage, from, to))`; `upgrades/` of the `strata:storage` slot with the bricks `Sort`, `Filter`, `Compress`, `Void`, `Magnet(radius, period)`, carried by an item `upgrade: Some(...)`. Bags: item field `bag: Some(Bag(placed: Some(block)))` (the slots and upgrade slots of the block put down, family `strata:backpack`) or `bag: Some(Bag(slots: 9, accepts: Some(tag)))` (a pouch); `equip: Some(Back)` to wear it; bricks `Pickup`, `Feed`, `Crafting`, `Tank(buckets)`.
