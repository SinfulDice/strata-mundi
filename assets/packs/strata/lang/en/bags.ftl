# Bags: backpacks, pouches and satchels, their upgrades.
block-strata-placed_leather_backpack = Leather Backpack
block-strata-placed_iron_backpack = Iron-Buckled Backpack
block-strata-placed_gold_backpack = Gold-Buckled Backpack
block-strata-placed_diamond_backpack = Diamond-Buckled Backpack
item-strata-leather_backpack = Leather Backpack
item-strata-iron_backpack = Iron-Buckled Backpack
item-strata-gold_backpack = Gold-Buckled Backpack
item-strata-diamond_backpack = Diamond-Buckled Backpack
item-strata-iron_backpack_kit = Iron Backpack Kit
item-strata-gold_backpack_kit = Gold Backpack Kit
item-strata-diamond_backpack_kit = Diamond Backpack Kit
item-strata-pickup_upgrade = Pickup Upgrade
item-strata-feed_upgrade = Feeding Upgrade
item-strata-crafting_upgrade = Crafting Upgrade
item-strata-tank_upgrade = Tank Upgrade
item-strata-seed_pouch = Seed Pouch
item-strata-ore_pouch = Ore Satchel
item-strata-ammo_pouch = Ammunition Pouch
item-strata-potion_pouch = Potion Satchel
tag-strata-backpacks = backpacks
tag-strata-seeds = seeds and plants
tag-strata-ores = ores
tag-strata-ammo = ammunition
tag-strata-potions = potions
upgrade-strata-pickup = Pickup
upgrade-strata-feeding = Feeding
upgrade-strata-crafting = Built-in Crafting
upgrade-strata-tank = Tank
upgrade-effect-pickup = what you pick up goes in first
upgrade-effect-feed = feeds you from what it holds when you are hungry
upgrade-effect-crafting = a 3×3 crafting grid in its screen
upgrade-effect-tank = holds up to { $buckets } buckets of one liquid
bag-slots-line = { $slots } slots, { $upgrades ->
        [0] no upgrades
        [one] { $upgrades } upgrade slot
       *[other] { $upgrades } upgrade slots
    }
bag-pouch-line = Fills up on pickup: { $what }
bag-accepts-strata-seeds = flowers and mushrooms (seeds come with crops)
bag-accepts-strata-ores = ores, coal, diamonds and nuggets
bag-accepts-strata-ammo = ammunition (comes with ranged weapons)
bag-accepts-strata-potions = potions (come with alchemy)
bag-backpack-line = Right click in hand: open. Worn on the back: <key:Backpack>. Sneak + right click: put down.
bag-open-line = Right click in hand: open.
bag-holds-line = Holds { $count } items{ $upgrades ->
        [0] {""}
       *[other] , { $upgrades } upgrades
    }
ui-inventory-back = Back (backpack)
ui-bag-crafting = Crafting
ui-bag-tank = Tank
ui-bag-tank-holds = { $liquid }: { $buckets } / { $capacity } buckets
ui-bag-tank-empty = Empty (0 / { $capacity })
ui-bag-tank-use = Held bucket
ui-bag-tank-help = Full bucket: pours in. Empty bucket: fills up.
