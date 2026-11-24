# Bags: backpacks, pouches and satchels, their upgrades.
block-strata-placed_leather_backpack = LEATHR BAKPAK
block-strata-placed_iron_backpack = IRON BUKL BAKPAK
block-strata-placed_gold_backpack = GOLD BUKL BAKPAK
block-strata-placed_diamond_backpack = SHINY BUKL BAKPAK
item-strata-leather_backpack = LEATHR BAKPAK
item-strata-iron_backpack = IRON BUKL BAKPAK
item-strata-gold_backpack = GOLD BUKL BAKPAK
item-strata-diamond_backpack = SHINY BUKL BAKPAK
item-strata-iron_backpack_kit = IRON BAKPAK KIT
item-strata-gold_backpack_kit = GOLD BAKPAK KIT
item-strata-diamond_backpack_kit = SHINY BAKPAK KIT
item-strata-pickup_upgrade = GRABBY UPGRAYD
item-strata-feed_upgrade = NOMS UPGRAYD
item-strata-crafting_upgrade = CRAFTY UPGRAYD
item-strata-tank_upgrade = SPLASHY UPGRAYD
item-strata-seed_pouch = SEEDZ POUCH
item-strata-ore_pouch = ROCKZ SATCHL
item-strata-ammo_pouch = PEW PEW POUCH
item-strata-potion_pouch = POSHUN SATCHL
tag-strata-backpacks = BAKPAKZ
tag-strata-seeds = SEEDZ AN PLANTZ
tag-strata-ores = ROCKZ
tag-strata-ammo = PEW PEW STUFF
tag-strata-potions = POSHUNZ
upgrade-strata-pickup = GRABBY
upgrade-strata-feeding = NOMS
upgrade-strata-crafting = POCKET CRAFTY
upgrade-strata-tank = SPLASHY
upgrade-effect-pickup = STUFF U GRAB GOEZ IN FURST
upgrade-effect-feed = FEEDZ U WHEN U HUNGRY
upgrade-effect-crafting = 3×3 CRAFTY GRID IN DA BAG
upgrade-effect-tank = HOLDZ { $buckets } BUKKITZ OF WETZ
bag-slots-line = { $slots } SPOTZ, { $upgrades ->
        [0] NO UPGRAYDZ
        [one] { $upgrades } UPGRAYD SPOT
       *[other] { $upgrades } UPGRAYD SPOTZ
    }
bag-pouch-line = FILLZ ITSELF WIF: { $what }
bag-accepts-strata-seeds = FLOWRZ AN SHROOMZ (SEEDZ COMIN WIF FARMZ)
bag-accepts-strata-ores = ROCKZ, COAL, SHINIEZ AN NUGGITZ
bag-accepts-strata-ammo = PEW PEW STUFF (COMIN WIF BOWZ)
bag-accepts-strata-potions = POSHUNZ (COMIN WIF MAGIK SOUP)
bag-backpack-line = RITE CLIK IN PAW: OPEN. ON BAK: <key:Backpack>. SNEEK + RITE CLIK: PUT DOWN.
bag-open-line = RITE CLIK IN PAW: OPEN.
bag-holds-line = HAZ { $count } STUFFZ{ $upgrades ->
        [0] {""}
       *[other] , { $upgrades } UPGRAYDZ
    }
ui-inventory-back = BAK (BAKPAK)
ui-bag-crafting = CRAFTY
ui-bag-tank = SPLASHY
ui-bag-tank-holds = { $liquid }: { $buckets } / { $capacity } BUKKITZ
ui-bag-tank-empty = NO WETZ (0 / { $capacity })
ui-bag-tank-use = HELD BUKKIT
ui-bag-tank-help = FULL BUKKIT: POURZ IN. EMPTY BUKKIT: FILLZ UP.
