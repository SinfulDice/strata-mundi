command-empty = Tapez une commande (/help pour la liste).
command-usage = Utilisation : { $usage }
command-unknown = Commande inconnue : { $name } (/help pour la liste).
command-needs-world = Cette commande s'utilise dans un monde.
command-cinematic-unknown = Cinématique inconnue.
command-tp-done = Téléporté en { $x } { $y } { $z }.
command-give-done = { $count } × { $item } dans l'inventaire.
command-give-unknown = Objet inconnu : { $item }
command-gamemode-done = Mode créatif.
command-gamemode-survival = Mode survie.
command-time-now = Heure : { $ticks } sur { $length } (jour { $day }).
command-daycycle-now = Cycle jour/nuit : { $cycle ->
        [on] l'heure avance
       *[off] l'heure est figée
    }.
command-fire-now = Feu : { $spread ->
        [on] se propage
       *[off] ne se propage pas
    }, { $destroy ->
        [on] détruit ce qui brûle
       *[off] ne détruit rien
    }.
command-seed-is = Graine : { $seed }
command-fluids-now = Fluides : { $flow ->
        [on] ils coulent
       *[off] figés
    }, { $budget } mises à jour par tick au plus.
command-fluids-budget = Budget entre 1 et { $max } mises à jour par tick.
command-help-usage = /help [commande]
command-help-help = Liste les commandes, ou détaille l'une d'elles.
command-tp-usage = /tp <x> <y> <z>  (~ = votre position)
command-tp-help = Téléporte (~5 = 5 blocs plus loin).
command-give-usage = /give <objet> [nombre]
command-give-help = Donne des objets (ce qui ne tient pas tombe au sol).
command-gamemode-usage = /gamemode <creative|survival>
command-gamemode-help = Change le mode de jeu.
command-time-usage = /time <set|add|query> [valeur]
command-time-help = Règle l'heure : set va jusqu'au prochain day, noon, night, midnight ou nombre de ticks ; add avance de ce nombre de ticks.
command-seed-usage = /seed
command-seed-help = Affiche la graine du monde.
command-fluids-usage = /fluids [on|off|query|<budget>]
command-fluids-help = Fait couler ou fige l'eau et la lave ; un nombre règle les mises à jour par tick, partagées avec le feu (un pas de feu en compte trois).
command-daycycle-usage = /daycycle [on|off|query]
command-daycycle-help = Fait avancer l'heure ou la fige.
command-fire-usage = /fire [spread|destroy] [on|off] ou /fire query
command-fire-help = Laisse le feu se propager ou non, détruire ce qui brûle ou non.
command-cinematic-usage = /cinematic <id>
command-cinematic-help = Joue une cinématique des packs.
ui-console-placeholder = Tapez une commande…
ui-console-help = <key:Complete> : compléter
command-not-allowed = Les commandes de triche sont désactivées dans ce monde.
