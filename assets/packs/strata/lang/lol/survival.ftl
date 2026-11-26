# SURVIVL BARZ (v0.3, milestone 8)
item-strata-berries = BERRIZ
item-strata-apple = APUL
item-strata-baked_apple = HOT APUL
item-strata-roasted_mushroom = CRISPY SHROOM
item-strata-bowl = BOWL
item-strata-mushroom_stew = SHROOM SOUP
item-strata-glass_bottle = EMPTEE BOTTL
item-strata-water_bottle = WATR BOTTL
item-strata-salt_water_bottle = SALTY WATR BOTTL
item-strata-waterskin = SKIN BAG
item-strata-waterskin_of_water = SKIN BAG OV WATR
item-strata-waterskin_of_salt_water = SKIN BAG OV SALTY WATR
item-strata-bandage = OUCHIE WRAP
item-strata-medical_kit = BOO-BOO BOX
category-strata-food = NOMZ
category-strata-care = OUCHIE FIXIN
item-stat-strata-fire_resistance = % NOT-HOT
effect-strata-bleeding = LEAKIN
effect-strata-healing = FIXIN
effect-strata-burning = ON FIRE LOL
damage-strata-fall = FALLED
damage-strata-drowning = 2 MUCH WATR
damage-strata-suffocation = NO AIRZ
damage-strata-starvation = HUNGRY
damage-strata-dehydration = THIRSTY
damage-strata-lava = HOT GOO
damage-strata-fire = HOTZ
damage-strata-bleeding = LEAKIN
damage-strata-burning = STILL HOTZ
survival-food-line = NOMZ { $food } (FULLZ { $saturation })
survival-thirst-line = SLURPZ { $thirst }
survival-salty-line = MOAR THIRSTY ({ $thirst }): BOIL IT IN TEH HOT BOX
survival-effect-line = GIVEZ: { $effect } ({ $seconds } S)
survival-cure-line = STOPZ: { $effect }
survival-sips-line = { $sips } SLURPZ
survival-fill-line = FILL AT WATR (SALTY BY TEH SEA)
hud-effect = { $effect } { $seconds } S
hud-effects-sample = FIXIN 6 S
hud-fainted = U FAINTED… AN WOKED UP AT TEH START. STUFF STILL THAR.
subtitle-eating = SUMWAN NOMS
subtitle-drinking = SUMWAN SLURPS
subtitle-bandaging = WRAP WRAP
subtitle-hurt = OWIE
ui-new-world-hunger = HUNGRY: { $on ->
        [on] YUS
       *[off] NOPE
    }
ui-new-world-thirst = THIRSTY: { $on ->
        [on] YUS
       *[off] NOPE
    }
ui-new-world-stamina = HUFF PUFF: { $on ->
        [on] YUS
       *[off] NOPE
    }
ui-new-world-survival-rate = HUNGRY N THIRSTY: { $rate } % FAST
option-hurt-shake = SHAKEZ WHEN OUCH
option-hurt-shake-help = TEH VIEW WOBBLEZ WHEN U GET HIT (OFF WIF "LESS WOBBLEZ").
option-hurt-flash = RED FLASH WHEN OUCH
option-hurt-flash-help = TEH SCREEN GOEZ RED WHEN U GET HIT.
option-show-mana = EMPTEE MAGIC BAR
option-show-mana-help = SHOW TEH MAGIC BAR B4 MAGIC IZ HERE.
hud-effect-separator = {" · "}
ui-percent = { $value } %
# Temperature (v0.3, milestone 8)
damage-strata-freezing = FROZED
damage-strata-overheating = 2 HOT
item-stat-strata-heat_insulation = NO HOT
item-stat-strata-cold_resistance = % NO FROZE
temperature-zone-freezing = SO FROZE
temperature-zone-cold = COLDZ
temperature-zone-comfortable = COZY
temperature-zone-hot = HOTZ
temperature-zone-scorching = SO SPICY
hud-temperature = { $zone } { $degrees } °C
hud-temperature-wet = WETZ
hud-temperature-hint-freezing = U IZ FROZIN AN OUCH: FIRE, ROOF OR WOOLZ, QUIK!
hud-temperature-hint-cold = IZ COLD: U SLOW AN GET HUNGRIER. FIRE, ROOF OR WOOLZ HALP.
hud-temperature-hint-hot = IZ HOT: U GET THIRSTY AN TIRED. FIND SHADE OR WATR.
hud-temperature-hint-scorching = TEH HOT HURTZ: GO AWAI FROM FIRE, IN SHADE OR WATR!
hud-temperature-hint-comfortable = IZ COZY.
ui-new-world-temperature = HOT AN COLD: { $on ->
        [on] YIS
       *[off] NOPE
    }
option-temperature-edges = FROST AN HOT ON EDGEZ
option-temperature-edges-help = TEH SCREEN EDGEZ GO FROSTY WEN U COLD AN GLOWY WEN U HOT.
