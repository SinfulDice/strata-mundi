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

## For pack makers

Folders `storage_tiers/` (`StorageTier(rows, upgrade_slots, upgrade_family)`) and `compression/` (`Compression(from, to, count: 9)`); block field `container: Some(Container(joins, upgradable: Some(Upgradable(family, tier))))`; kit `kit: Some(Kit(on: Storage, from, to))`; `upgrades/` of the `strata:storage` slot with the bricks `Sort`, `Filter`, `Compress`, `Void`, `Magnet(radius, period)`, carried by an item `upgrade: Some(...)`.
