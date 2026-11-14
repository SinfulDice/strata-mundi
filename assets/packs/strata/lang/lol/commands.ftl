command-empty = TYPE A COMMANDZ (/help 4 LIST).
command-usage = HOW 2 USE: { $usage }
command-unknown = WUT IZ { $name }? (/help 4 LIST)
command-needs-world = NEEDZ A WORLD 4 DAT.
command-cinematic-unknown = NO SUCH MOOVEE.
command-tp-done = ZOOPED 2 { $x } { $y } { $z }.
command-give-done = { $count } × { $item } IN UR STUFFZ. U WELCOME.
command-give-unknown = WUT IZ { $item }?
command-gamemode-done = CRAYTIV MODE.
command-gamemode-survival = SURVIVL MODE. GUD LUK.
command-time-now = TIEM: { $ticks } OF { $length } (DAI { $day }).
command-daycycle-now = DAI AN NITE: { $cycle ->
        [on] TIEM GOEZ ON
       *[off] TIEM IZ STUCK
    }.
command-fire-now = HOT FIRE: { $spread ->
        [on] GOEZ EVRYWHER
       *[off] STAYZ PUT
    }, { $destroy ->
        [on] NOMZ WUT BURNZ
       *[off] NOMZ NOTHIN
    }.
command-seed-is = SEED: { $seed }
command-fluids-now = WATR AN LAVA: { $flow ->
        [on] GOEZ SPLOOSH
       *[off] STUK IN PLAIS
    }, { $budget } UPDAETZ PER TICK MAX.
command-fluids-budget = BUDGIT IZ 1 TO { $max } UPDAETZ PER TICK.
command-help-usage = /help [command]
command-help-help = LISTZ ALL TEH COMMANDZ.
command-tp-usage = /tp <x> <y> <z>  (~ = WER U R)
command-tp-help = ZOOPZ U (~5 = 5 BLOKZ MOAR).
command-give-usage = /give <item> [HOW MANY]
command-give-help = GIVEZ STUFFZ (TOO MUCH FALLZ ON TEH FLOOR).
command-gamemode-usage = /gamemode <creative|survival>
command-gamemode-help = CHANGEZ TEH GAEM MODE.
command-time-usage = /time <set|add|query> [value]
command-time-help = SETZ TEH TIEM: set GOEZ 2 NEXT day, noon, night, midnight OR TICKZ; add SKIPZ DAT MANY TICKZ.
command-seed-usage = /seed
command-seed-help = SHOWZ TEH WORLD SEED.
command-fluids-usage = /fluids [on|off|query|<budget>]
command-fluids-help = MAEKZ WATR AN LAVA GO SPLOOSH OR STOP; A NUMBR SETZ HOW MANY UPDAETZ PER TICK, SHARD WIF FIER (1 FIER STEP IZ 3).
command-daycycle-usage = /daycycle [on|off|query]
command-daycycle-help = MAEKZ TIEM GO OR STOP.
command-fire-usage = /fire [spread|destroy] [on|off] OR /fire query
command-fire-help = LETZ HOT FIRE GO EVRYWHER OR NOT, AN NOM BLOKZ OR NOT.
command-cinematic-usage = /cinematic <id>
command-cinematic-help = PLAYZ A MOOVEE.
ui-console-placeholder = TYPE A COMMANDZ…
ui-console-help = <key:Complete>: FINISH 4 ME
command-not-allowed = NO CHEATZ IN DIS WURLD.
