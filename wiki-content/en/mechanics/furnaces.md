## Principle

A **furnace** cooks the item on top with the fuel below, and the result appears on the right. The flame shows the fuel left, the arrow the cooking under way. It keeps working with its screen closed, and its front glows while the fire burns. The **blast furnace** only smelts ores and the **smoker** only cooks food, twice as fast.

## Fuels

Coal and charcoal: 8 items in a stone furnace; lava bucket: 100 (the bucket comes back); wood, planks, crafting table, chest: one and a half; stick: a half. The fire only starts when there is something to cook and room in the output.

## Upgradable furnaces

A **kit** used (right click) on a furnace takes it to the next tier **in place**: stone → iron → gold → diamond. Contents, fuel and the cooking under way are kept. Each tier cooks faster (10, 8, 5 then 3 s), gets more from a coal (8, 10, 12, 16 items) and gains upgrade slots (0 to 3) for **modules**: speed, fuel saving, double output. Two identical modules do not add up. The same kit works on smokers and blast furnaces.

## Stonecutter

Lay a block in its cell, choose a variant in the list (stone bricks, polished stone, cut sandstone…) and take it. No fuel, nothing lost: one block gives one block.

## For pack makers

Folders `cooking/` (`Cooking(ingredient, result, time, experience, kind)`), `fuels/` (`Fuel(ingredient, burn_time)`), `stonecutting/` (`Cutting(ingredient, result)`), `furnace_tiers/` (`FurnaceTier(cook_time, fuel_value, upgrade_slots)`); block fields `furnace: Some(Furnace(family, tier, kinds, speed))` and `placement: Facing(lit: Some(Lit(front, light_emission)))`; item fields `kit: Some(Kit(from, to))` and `module: Some(Furnace(cook_time, fuel_value, output))`.
