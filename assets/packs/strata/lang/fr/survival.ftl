# Jauges de survie (v0.3, jalon 8)
item-strata-berries = Baies
item-strata-apple = Pomme
item-strata-baked_apple = Pomme cuite
item-strata-roasted_mushroom = Champignon grillé
item-strata-bowl = Bol
item-strata-mushroom_stew = Ragoût de champignons
item-strata-glass_bottle = Bouteille en verre
item-strata-water_bottle = Bouteille d'eau
item-strata-salt_water_bottle = Bouteille d'eau salée
item-strata-waterskin = Gourde en cuir
item-strata-waterskin_of_water = Gourde d'eau
item-strata-waterskin_of_salt_water = Gourde d'eau salée
item-strata-bandage = Bandage
item-strata-medical_kit = Trousse de soin
category-strata-food = Nourriture
category-strata-care = Soins
item-stat-strata-fire_resistance = % de résistance au feu
effect-strata-bleeding = Saignement
effect-strata-healing = Soin
effect-strata-burning = En feu
damage-strata-fall = Chute
damage-strata-drowning = Noyade
damage-strata-suffocation = Suffocation
damage-strata-starvation = Faim
damage-strata-dehydration = Soif
damage-strata-lava = Lave
damage-strata-fire = Feu
damage-strata-bleeding = Saignement
damage-strata-burning = Brûlure
survival-food-line = Nourrit de { $food } (rassasie de { $saturation })
survival-thirst-line = Désaltère de { $thirst }
survival-salty-line = Donne soif ({ $thirst }) : à faire bouillir au four ou au fumoir
survival-effect-line = Donne : { $effect } ({ $seconds } s)
survival-cure-line = Arrête : { $effect }
survival-sips-line = { $sips } gorgées
survival-fill-line = Se remplit à une source d'eau (salée au bord de la mer)
hud-effect = { $effect } { $seconds } s
hud-effects-sample = Soin 6 s
hud-fainted = Vous vous êtes évanoui… et réveillé au point de départ, vos affaires avec vous.
subtitle-eating = Quelqu'un mange
subtitle-drinking = Quelqu'un boit
subtitle-bandaging = Bandage qu'on enroule
subtitle-hurt = Aïe
ui-new-world-hunger = Faim : { $on ->
        [on] oui
       *[off] non
    }
ui-new-world-thirst = Soif : { $on ->
        [on] oui
       *[off] non
    }
ui-new-world-stamina = Endurance : { $on ->
        [on] oui
       *[off] non
    }
ui-new-world-survival-rate = Faim et soif : vitesse { $rate } %
option-hurt-shake = Secousse quand on est blessé
option-hurt-shake-help = La vue tremble un instant à chaque coup reçu (coupée avec « Réduire les effets caméra »).
option-hurt-flash = Éclair rouge quand on est blessé
option-hurt-flash-help = L'écran rougit un instant à chaque coup reçu.
option-show-mana = Barre de mana vide
option-show-mana-help = Afficher la barre de mana avant que la magie n'arrive.
hud-effect-separator = {" · "}
ui-percent = { $value } %
# Température (v0.3, jalon 8)
damage-strata-freezing = Gel
damage-strata-overheating = Coup de chaleur
item-stat-strata-heat_insulation = d'isolation contre la chaleur
item-stat-strata-cold_resistance = % de résistance au gel
temperature-zone-freezing = Glacial
temperature-zone-cold = Froid
temperature-zone-comfortable = Agréable
temperature-zone-hot = Chaud
temperature-zone-scorching = Brûlant
hud-temperature = { $zone } { $degrees } °C
hud-temperature-wet = Mouillé
hud-temperature-hint-freezing = Vous gelez et perdez de la vie : un feu, un abri ou de la laine, vite !
hud-temperature-hint-cold = Il fait froid : vous ralentissez et avez faim plus vite. Feu, abri ou laine aident.
hud-temperature-hint-hot = Il fait chaud : vous avez soif plus vite et soufflez moins bien. Cherchez l'ombre ou l'eau.
hud-temperature-hint-scorching = La chaleur vous blesse : éloignez-vous du feu, mettez-vous à l'ombre ou dans l'eau !
hud-temperature-hint-comfortable = La température est agréable.
ui-new-world-temperature = Température : { $on ->
        [on] oui
       *[off] non
    }
option-temperature-edges = Givre et chaleur sur les bords
option-temperature-edges-help = Les bords de l'écran se couvrent de givre quand on a froid et rougeoient quand on a chaud.
