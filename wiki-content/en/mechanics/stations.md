## One station per material

Each material is worked at its **work station**: a block you place, then open with a right click.

- **Crafting table**: the 3×3 crafting grid (tools, weapons, blocks, stations).
- **Furnaces** (furnace, smoker, blast furnace): cooking and smelting ores into ingots, with fuel.
- **Stonecutter**: stone variants, one by one.
- **Forge**: ingots become mail rings, armour plates and mobility boots; each metal piece burns one coal. Diamonds are cut into scales there.
- **Tannery**: leather and leather armour.
- **Loom**: cloth, wool, cloth clothing and the wool cloak.
- **Jeweller**: rings, amulets and belts.
- **Anvil**: upgrades and repairs.

## Benches with a list

The forge, the tannery, the loom and the jeweller work the same way: a **list** of what they can make, **four cells** and the **result**. Choose an item: its ingredients go from the inventory to the cells; the tooltip says what it takes and whether you lack something. Ingredients can also be laid by hand. Each bench has its own screen: its arrangement, its colour and its help. With a gamepad, the d-pad moves from cell to cell; the "<" ">" arrows scroll a long list.

## For pack makers

A block `station: Some(Bench("mypack:bench"))`, recipes `bench_recipes/<name>.ron` (`BenchRecipe(bench, ingredients: [(Item(..), n)…], result)`, one to four kinds of ingredients) and, if wanted, the look of its screen in `benches/<name>.ron`: `Bench(layout: Forge, accent: "#7a3b1e")` (arrangements: `Plain`, `Jeweller`, `Forge`, `Tannery`, `Loom`). The screen's help is the translation `bench-<namespace>-<name>`.
