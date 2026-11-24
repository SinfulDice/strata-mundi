# Sacs : sacs à dos, bourses et sacoches, leurs améliorations.
block-strata-placed_leather_backpack = Sac à dos en cuir
block-strata-placed_iron_backpack = Sac à dos bouclé de fer
block-strata-placed_gold_backpack = Sac à dos bouclé d'or
block-strata-placed_diamond_backpack = Sac à dos bouclé de diamant
item-strata-leather_backpack = Sac à dos en cuir
item-strata-iron_backpack = Sac à dos bouclé de fer
item-strata-gold_backpack = Sac à dos bouclé d'or
item-strata-diamond_backpack = Sac à dos bouclé de diamant
item-strata-iron_backpack_kit = Kit de sac à dos en fer
item-strata-gold_backpack_kit = Kit de sac à dos en or
item-strata-diamond_backpack_kit = Kit de sac à dos en diamant
item-strata-pickup_upgrade = Amélioration de ramassage
item-strata-feed_upgrade = Amélioration de nourrissage
item-strata-crafting_upgrade = Amélioration d'établi
item-strata-tank_upgrade = Amélioration de réservoir
item-strata-seed_pouch = Bourse à graines
item-strata-ore_pouch = Sacoche à minerais
item-strata-ammo_pouch = Bourse à munitions
item-strata-potion_pouch = Sacoche à potions
tag-strata-backpacks = sacs à dos
tag-strata-seeds = graines et plantes
tag-strata-ores = minerais
tag-strata-ammo = munitions
tag-strata-potions = potions
upgrade-strata-pickup = Ramassage
upgrade-strata-feeding = Nourrissage
upgrade-strata-crafting = Établi intégré
upgrade-strata-tank = Réservoir
upgrade-effect-pickup = ce que vous ramassez y entre d'abord
upgrade-effect-feed = vous nourrit de ce qu'il contient quand vous avez faim
upgrade-effect-crafting = une grille de fabrication 3×3 dans son écran
upgrade-effect-tank = garde jusqu'à { $buckets } seaux d'un liquide
bag-slots-line = { $slots } cases, { $upgrades ->
        [0] sans amélioration
        [one] { $upgrades } emplacement d'amélioration
       *[other] { $upgrades } emplacements d'amélioration
    }
bag-pouch-line = Se remplit au ramassage : { $what }
bag-accepts-strata-seeds = fleurs et champignons (les graines viendront avec les cultures)
bag-accepts-strata-ores = minerais, charbon, diamants et pépites
bag-accepts-strata-ammo = munitions (viendront avec les armes à distance)
bag-accepts-strata-potions = potions (viendront avec l'alchimie)
bag-backpack-line = Clic droit en main : ouvrir. Porté sur le dos : <key:Backpack>. Accroupi + clic droit : poser.
bag-open-line = Clic droit en main : ouvrir.
bag-holds-line = Contient { $count } objets{ $upgrades ->
        [0] {""}
       *[other] , { $upgrades } améliorations
    }
ui-inventory-back = Dos (sac à dos)
ui-bag-crafting = Établi
ui-bag-tank = Réservoir
ui-bag-tank-holds = { $liquid } : { $buckets } / { $capacity } seaux
ui-bag-tank-empty = Vide (0 / { $capacity })
ui-bag-tank-use = Seau tenu
ui-bag-tank-help = Seau plein : il s'y vide. Seau vide : il s'y remplit.
