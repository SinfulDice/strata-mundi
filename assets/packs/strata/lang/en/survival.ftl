# Survival gauges (v0.3, milestone 8)
item-strata-berries = Berries
item-strata-apple = Apple
item-strata-baked_apple = Baked Apple
item-strata-roasted_mushroom = Roasted Mushroom
item-strata-bowl = Bowl
item-strata-mushroom_stew = Mushroom Stew
item-strata-glass_bottle = Glass Bottle
item-strata-water_bottle = Bottle of Water
item-strata-salt_water_bottle = Bottle of Salt Water
item-strata-waterskin = Leather Waterskin
item-strata-waterskin_of_water = Waterskin of Water
item-strata-waterskin_of_salt_water = Waterskin of Salt Water
item-strata-bandage = Bandage
item-strata-medical_kit = Medical Kit
category-strata-food = Food
category-strata-care = Care
item-stat-strata-fire_resistance = % fire resistance
effect-strata-bleeding = Bleeding
effect-strata-healing = Healing
effect-strata-burning = Burning
damage-strata-fall = Fall
damage-strata-drowning = Drowning
damage-strata-suffocation = Suffocation
damage-strata-starvation = Hunger
damage-strata-dehydration = Thirst
damage-strata-lava = Lava
damage-strata-fire = Fire
damage-strata-bleeding = Bleeding
damage-strata-burning = Burning
survival-food-line = Feeds { $food } (saturation { $saturation })
survival-thirst-line = Quenches { $thirst }
survival-salty-line = Makes thirsty ({ $thirst }): boil it in a furnace or a smoker
survival-effect-line = Gives: { $effect } ({ $seconds } s)
survival-cure-line = Stops: { $effect }
survival-sips-line = { $sips } sips
survival-fill-line = Fills at a water source (salty by the sea)
hud-effect = { $effect } { $seconds } s
hud-effects-sample = Healing 6 s
hud-fainted = You fainted… and woke up at the starting point, your things with you.
subtitle-eating = Someone eats
subtitle-drinking = Someone drinks
subtitle-bandaging = Bandage wound round
subtitle-hurt = Ouch
ui-new-world-hunger = Hunger: { $on ->
        [on] on
       *[off] off
    }
ui-new-world-thirst = Thirst: { $on ->
        [on] on
       *[off] off
    }
ui-new-world-stamina = Stamina: { $on ->
        [on] on
       *[off] off
    }
ui-new-world-survival-rate = Hunger and thirst: { $rate } % speed
option-hurt-shake = Shake when hurt
option-hurt-shake-help = The view trembles for a moment at each hit (off with "Reduce camera effects").
option-hurt-flash = Red flash when hurt
option-hurt-flash-help = The screen turns red for a moment at each hit.
option-show-mana = Empty mana bar
option-show-mana-help = Show the mana bar before magic arrives.
hud-effect-separator = {" · "}
ui-percent = { $value } %
# Temperature (v0.3, milestone 8)
damage-strata-freezing = Freezing
damage-strata-overheating = Heatstroke
item-stat-strata-heat_insulation = heat insulation
item-stat-strata-cold_resistance = % freezing resistance
temperature-zone-freezing = Freezing
temperature-zone-cold = Cold
temperature-zone-comfortable = Pleasant
temperature-zone-hot = Hot
temperature-zone-scorching = Scorching
hud-temperature = { $zone } { $degrees } °C
hud-temperature-wet = Wet
hud-temperature-hint-freezing = You are freezing and losing health: a fire, a shelter or wool, quick!
hud-temperature-hint-cold = It is cold: you slow down and get hungry faster. Fire, shelter or wool help.
hud-temperature-hint-hot = It is hot: you get thirsty faster and tire sooner. Find shade or water.
hud-temperature-hint-scorching = The heat hurts: get away from the fire, into the shade or the water!
hud-temperature-hint-comfortable = The temperature is pleasant.
ui-new-world-temperature = Temperature: { $on ->
        [on] on
       *[off] off
    }
option-temperature-edges = Frost and heat at the edges
option-temperature-edges-help = The edges of the screen frost over when you are cold and glow when you are hot.
