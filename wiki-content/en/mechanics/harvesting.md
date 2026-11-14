## Principle

In creative, one click breaks a block. In **survival**, **hold** the attack button (left click, or the right trigger on a gamepad) on the block: **cracks** appear and grow in ten steps, with small knocks and chips, until the block gives way. Looking away, letting go of the button or changing the held item stops the break and clears the cracks.

## Time

The time depends on the block's **hardness** and the **tool** held: each block has a favourite kind of tool (pickaxe, axe, shovel, shears). By hand, a block takes its hardness × 1.5 seconds (dirt: 0.75 s); the right tool is faster, according to its speed. Each block's page gives its hardness, its tool and its time by hand.

## Required tool

Some blocks (stone, ores, obsidian…) **need** a tool: without it, or with a tool whose **level** is too low, breaking is very slow (hardness × 5 seconds) and **gives nothing**. The item's tooltip says so ("Needs a tool: pickaxe of level 2 or more"). Levels: 0 wood, 1 stone, 2 iron, 3 diamond.

## Loot

A block broken in survival leaves its loot on the ground. By default it is the block itself; some give something else, described by a **loot table** of the pack: grass gives dirt, leaves and tall grass are only kept with shears, mushroom blocks sometimes give mushrooms, glass and ice shatter. The rolls follow the world seed.

## For pack makers

Block fields `tool: Some(BlockTool(kind: "strata:pickaxe", level: 1, required: true))` and `loot: Some("strata:my_table")`; tables in `loot_tables/` (pools, weighted entries, counts, conditions `Tool`, `Not`, `Chance`); item field `tool: Some(Tool(kinds: ["strata:pickaxe"], level: 2, speed: 6.0))`.
