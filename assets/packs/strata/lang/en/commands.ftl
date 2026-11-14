command-empty = Type a command (/help for the list).
command-usage = Usage: { $usage }
command-unknown = Unknown command: { $name } (/help for the list).
command-needs-world = This command works inside a world.
command-cinematic-unknown = Unknown cinematic.
command-tp-done = Teleported to { $x } { $y } { $z }.
command-give-done = { $count } × { $item } in the inventory.
command-give-unknown = Unknown item: { $item }
command-gamemode-done = Creative mode.
command-gamemode-survival = Survival mode.
command-time-now = Time: { $ticks } out of { $length } (day { $day }).
command-daycycle-now = Day and night: { $cycle ->
        [on] time goes on
       *[off] time stands still
    }.
command-fire-now = Fire: { $spread ->
        [on] spreads
       *[off] does not spread
    }, { $destroy ->
        [on] destroys what burns
       *[off] destroys nothing
    }.
command-seed-is = Seed: { $seed }
command-fluids-now = Liquids: { $flow ->
        [on] flowing
       *[off] frozen
    }, { $budget } updates per tick at most.
command-fluids-budget = The budget goes from 1 to { $max } updates per tick.
command-help-usage = /help [command]
command-help-help = Lists the commands, or details one.
command-tp-usage = /tp <x> <y> <z>  (~ = your position)
command-tp-help = Teleports (~5 = 5 blocks further).
command-give-usage = /give <item> [count]
command-give-help = Gives items (what does not fit falls to the ground).
command-gamemode-usage = /gamemode <creative|survival>
command-gamemode-help = Changes the game mode.
command-time-usage = /time <set|add|query> [value]
command-time-help = Sets the time: set goes on to the next day, noon, night, midnight or tick count; add moves on by that many ticks.
command-seed-usage = /seed
command-seed-help = Shows the world seed.
command-fluids-usage = /fluids [on|off|query|<budget>]
command-fluids-help = Lets water and lava flow or freezes them; a number sets the updates per tick, shared with fire (a fire step counts as three).
command-daycycle-usage = /daycycle [on|off|query]
command-daycycle-help = Lets the time of day go on or stops it.
command-fire-usage = /fire [spread|destroy] [on|off] or /fire query
command-fire-help = Lets fire spread or not, and destroy what burns or not.
command-cinematic-usage = /cinematic <id>
command-cinematic-help = Plays a cinematic from the packs.
ui-console-placeholder = Type a command…
ui-console-help = <key:Complete>: complete
command-not-allowed = Cheating commands are off in this world.
