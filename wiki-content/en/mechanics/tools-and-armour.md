## Principle

Tools break the blocks of their kind faster and harvest those that need them. Each tool belongs to a material **tier**: **wood, stone, iron, gold, diamond** (more will come after diamond). The tier gives the **mining level** (0 wood, 1 stone, 2 iron and gold, 3 diamond), the **speed**, the number of **uses**, the **damage** added to weapons and the **upgrade slots**. Diamond needs an iron pickaxe, obsidian a diamond one. Gold is very fast but wears out quickly. Each item's page gives its numbers.

## Tools

- **Pickaxe, axe, shovel**: stone and ores, wood, dirt and sand.
- **Hoe**: a right click on dirt or grass open to the air tills it into farmland.
- **Sword**: keeps its damage and pace for combat, which comes later.
- **Shears**: keep leaves, vines and tall grass.
- **Sickle**: cutting a plant also cuts those around it, up to two blocks, for a single use.
- **Chisel**: a right click on stone bricks carves them; each stroke moves on to the next pattern, then back to plain bricks.
- **Multi-tool**: pickaxe, axe and shovel at once, at iron level, but less durable.

Wooden and stone tools are known from the start.

## Armour

Four slots: head, chest, legs, feet. Three **weights**:

- **light** (leather, cloth): little protection, no hindrance;
- **medium** (mail, scales): better protection, and will cost a little stamina and mana;
- **heavy** (plates): the most protection, but each piece **slows walking** by 2.5 % (10 % for the full set) and will cost more stamina and mana.

Iron and gold come as mail and plate, diamond as scales and plate. Mail is made of rings (one ingot gives two), diamond scales of a diamond (two scales). Total **protection** is shown in the inventory.

## Durability

Each use wears the tool; the bar under the slot shows what is left and the tooltip the exact number. When creating the world, the "Worn-out items" option chooses: the item **breaks** at zero, or it is **kept until repaired**, greyed out and useless (a broken tool is a bare hand, broken armour no longer protects).

## Provisional materials

Tall grass and ferns cut without shears sometimes leave a **plant fibre**. Three fibres make **cloth**; two fibres and a mud make, for now, **leather**, until animals come.

## For pack makers

Folders `tiers/` (`Tier(level, speed, durability, damage, enchantability, upgrade_slots, armor_durability)`), `upgrade_slots/` (`UpgradeSlot(max_per_item)`) and `armor_weights/` (`ArmorWeight(speed, stamina, mana)`, shares per piece); item fields `tier: "strata:iron"`, `tool: Some(Tool(kinds: [...], level, speed, sweep))` (the tier fills what is missing), `weapon: Some(Weapon(damage, attack_speed))`, `armor: Some(Armor(protection, weight))`, `upgrade_slots`, `action: Some(Work)`; block field `worked: {"strata:hoe": Worked(into: "strata:farmland", open_above: true)}`.
