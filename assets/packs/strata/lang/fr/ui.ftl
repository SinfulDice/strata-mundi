game-title = Strata Mundi
ui-menu-help = <key:MenuConfirm> : choisir    <key:MenuBack> : retour
ui-ok = OK
ui-back = Retour
ui-cancel = Annuler
ui-on = oui
ui-off = non

ui-title-play = Jouer
ui-title-options = Options
ui-title-quit = Quitter

ui-worlds-title = Mondes
ui-worlds-play = Jouer
ui-worlds-create = Nouveau
ui-worlds-delete = Supprimer
ui-worlds-empty = Aucun monde pour l'instant : créez-en un !
ui-world-detail = { $date } - { $mode } - { $time }
ui-world-too-old = Créé par une version plus ancienne du jeu
ui-world-too-new = Créé par une version plus récente du jeu
ui-world-broken = Description du monde illisible
ui-play-time-minutes = { $minutes } min
ui-play-time-hours = { $hours } h { $minutes }
ui-mode-creative = Créatif

ui-new-world-title = Nouveau monde
ui-new-world-default-name = Nouveau monde
ui-new-world-name = Nom du monde
ui-new-world-seed = Graine
ui-new-world-seed-random = (vide : au hasard)
ui-new-world-border = Bordure du monde (blocs depuis le centre)
ui-new-world-mode = Mode de jeu : { $mode }
ui-new-world-fluids = Fluides : { $flow ->
        [on] ils coulent
       *[off] figés
    }
ui-new-world-day-cycle = Cycle jour/nuit : { $cycle ->
        [on] activé
       *[off] heure figée
    }
ui-new-world-fire-spread = Feu : { $spread ->
        [on] se propage
       *[off] contenu
    }
ui-new-world-fire-destroy = Brûle : { $destroy ->
        [on] oui
       *[off] rien
    }
ui-new-world-create = Créer

ui-delete-question = Supprimer « { $name } » ?
ui-delete-warning = Les copies de sécurité sont gardées.
ui-delete-confirm = Supprimer

ui-error-incompatible = Ce monde vient d'une version incompatible.
ui-error-unavailable = Impossible d'ouvrir ce monde (voir le journal).

ui-pause-title = Pause
ui-pause-resume = Reprendre
ui-pause-options = Options
ui-pause-save-quit = Sauvegarder et quitter

ui-options-title = Options
ui-scale-auto = auto

ui-editor-title = Disposition du HUD
ui-editor-help-select = <key:Attack> : choisir
ui-editor-help-move = Flèches, glisser : déplacer
ui-editor-reset = Défaut
ui-editor-done = Fini
ui-editor-sample = Pierre

ui-key-erase = <-
ui-key-shift = Aa
ui-key-space = [ ]
ui-key-done = OK

ui-inventory-title = Inventaire créatif
ui-search = Rechercher…
ui-inventory-help = <key:Attack>, <key:MenuConfirm> ou 1-9 : vers la barre
ui-tab-all = Tout

key-escape = Échap
key-space = Espace
key-enter = Entrée
key-backspace = Retour arr.
key-tab = Tab
key-shift = Maj
key-ctrl = Ctrl
key-alt = Alt
key-up = Haut
key-down = Bas
key-left = Gauche
key-right = Droite
key-other = ?
key-mouse-left = Clic
key-mouse-right = Clic droit
key-mouse-middle = Clic molette
key-mouse-other = Souris
pad-south = B
pad-east = A
pad-north = X
pad-west = Y
pad-l1 = L
pad-r1 = R
pad-l2 = ZL
pad-r2 = ZR
pad-select = Select
pad-start = Start
pad-left-stick = L3
pad-right-stick = R3
pad-up = Haut
pad-down = Bas
pad-left = Gauche
pad-right = Droite
ui-error-broken = La description de ce monde est illisible.
ui-version = v{ $version }
ui-scale-times = ×{ $scale }
ui-crash-message = Le jeu s'est arrêté brutalement la dernière fois.
ui-crash-detail = Le rapport est dans le dossier crash-reports.
ui-crash-report = Signaler ce plantage
ui-crash-ignore = Ne pas signaler
ui-update-available = Nouvelle version { $version } disponible sur GitHub
ui-reload-failed = Ressources non rechargées : voir le journal.
ui-reload-blocks-changed = Blocs ou objets modifiés : relancez le jeu.
hud-dismount = <key:Sneak> : descendre du bateau

world-type-strata-normal = Normal
world-type-strata-flat = Plat
world-type-strata-amplified = Amplifié
world-type-strata-single_biome = Biome unique
world-type-strata-archipelago = Archipel
world-code-malformed = Ce code est abîmé ou n'est pas un code de monde.
world-code-future = Ce code vient d'une version plus récente du jeu.
world-settings-unknown-type = Type de monde inconnu : { $id } (pack de contenu manquant ?).
world-settings-unknown-biome = Biome inconnu : { $id }.
world-settings-unknown-block = Bloc inconnu : { $id }.
world-settings-no-layers = Un monde plat a besoin d'au moins une couche.

ui-new-world-customize = Personnaliser la génération…
ui-gen-title = Génération du monde
ui-gen-type = Type : { $type }
ui-gen-tab-general = Réglages
ui-gen-tab-layers = Couches
ui-gen-tab-biome = Biome
ui-gen-tab-code = Code
ui-gen-biome-size = Biomes : { $value }
ui-gen-sea-level = Niveau de la mer : { $value }
ui-gen-caves = Grottes : { $value }
ui-gen-ores = Minerais : { $value }
ui-gen-percent = { $value } %
ui-gen-default = { $value } (normal)
ui-gen-general-help = <key:MenuLeft> <key:MenuRight> : régler
ui-gen-thickness = × { $count }
ui-gen-block = Bloc : { $block }
ui-gen-layer-thickness = Épaisseur : { $count }
ui-gen-layer-add = Ajouter
ui-gen-layer-remove = Retirer
ui-gen-layer-up = Monter
ui-gen-layer-down = Baisser
ui-gen-code-current = Code de ces réglages :
ui-gen-copy = Copier
ui-gen-code-paste = Coller ou taper un code :
ui-gen-code-placeholder = S1-…
ui-gen-paste = Coller
ui-gen-apply = Appliquer
ui-gen-code-applied = Code appliqué.
ui-gen-preview-off = Aperçu désactivé
ui-gen-preview-busy = Calcul…
ui-gen-preview-legend = 2 km de côté
ui-gen-help = <key:HotbarPrevious> <key:HotbarNext> : onglets
ui-gen-done = Terminé
ui-gen-preview-failed = Carte impossible (voir le journal)
ui-gen-copied = Code copié.
ui-gen-no-clipboard = Presse-papiers indisponible : recopiez le code.
