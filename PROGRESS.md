# PROGRESS — Strata Mundi

## État

- **Version en cours** : v0.3 — Survie. Détail : `docs/versions/v0.3.md`.
- **Jalon en cours** : 7 — Rangement
  - [x] J1 Objets et inventaire (piles, durabilité, métadonnées, 27+9+4+1, glisser-déposer, tri, verrouillage, dépôt rapide, recherche, objets lâchés, coffres, tonneaux)
  - [x] J2 Récolte (temps de casse, fissures, tables de butin, outil requis)
  - [x] J3 Craft (2×2, établi, recettes en pack, livre de recettes, fours/fumoir/haut fourneau, tailleur de pierre, fours améliorables)
    - [x] J3a grille/recettes/livre (recettes et tags en pack, 2×2 et établi 3×3 côté serveur, Maj-clic, restes, recettes apprises et expérimentation, option « recettes à découvrir », journal avec livre de recettes)
    - [x] J3b fours, tailleur, fours améliorables (combustibles, cuisson et taille en pack, façade orientée et allumée, fours actifs seulement, kits sur place, modules, écrans, sauvegarde)
  - [x] J4 Outils, armes, armures (tiers, poids, outils spécialisés, marteau de mineur, durabilité, améliorations génériques, réparation)
    - [x] J4a tiers, outils, armes, armures, outils spécialisés, durabilité (tiers et poids d'armure en pack, pioche/hache/pelle/houe/épée par tier, cisailles, faucille, burin, outil multi-usage, armures légère/intermédiaire/lourde, protection dans l'inventaire, option « objets usés » ; armure portée non dessinée faute de modèle de joueur en 3e personne, D076)
    - Note pour J13 : les statistiques des infobulles (dégâts, protection, emplacements) passent toutes par `Content::stats_known` (toujours vrai) : y brancher le savoir.
    - [x] J4b marteau de mineur, améliorations génériques, enclume, réparation (améliorations en pack par briques, calcul unique des statistiques, non-cumul, enclume et son écran, réparation par matériau ou objet identique, marteaux à zone 3×1 → 5×5, contour et choix de zone ; XP affichée mais offerte jusqu'au J9 : `Game::can_pay_xp`/`pay_xp`, D082 ; pas d'usure d'enclume)
  - [x] J5 Accessoires et sets
    - [x] J5a emplacements d'accessoires (46 cases, migration), stats portées uniques (armure + accessoires + sets, non-cumul), sets en pack, mobilité (double saut, esquive, escalade) partagée serveur/client, option de monde, joaillier (établi générique `Bench`, `bench_recipes/`), accessoires et bottes, laine provisoire, wiki
    - [x] J5b captures (inventaire, infobulle d'ensemble fr/en/lol, joaillier, DS), docs (accessoires-et-sets.md, D086-D089, équilibrage), wiki
    - Note pour J6 : forge, tannerie, métier à tisser = établis `Bench` (D089) ; y déplacer les bottes de mobilité (armurier). Note pour J8 : lire `WornStat::MaxHealth`, `Breath`, `Insulation` dans `worn_stats`. Note pour J13 : recettes d'établi toutes connues.
  - [x] J6 Stations de travail spécialisées (forge avec charbon en ingrédient, tannerie, métier à tisser sur l'établi générique ; armures, anneaux, écailles, bottes de mobilité, cuir, tissu, laine, cape déplacés à leur station ; écran propre par établi : `benches/` (disposition, couleur), bandes, aide, liste qui défile ; page wiki « Stations de travail » ; D091-D092)
    - Note pour J7 : convertir d'abord les modules de four en améliorations (D085). Note pour v0.4 : cuir et laine provisoires déjà à la tannerie et au métier, remplacer les ingrédients par peaux et laine de mouton.
  - [ ] J7 Rangement (sac à dos, bourses, coffres et tonneaux améliorables)
  - [ ] J8 Jauges de survie (vie, faim, soif, température, endurance, dégâts, soins, HUD)
  - [ ] J9 Expérience
  - [ ] J10 Combat simple et mort (lit, tombe, messages, statistiques)
  - [ ] J11 Options de monde et modes (intégrité structurelle, gamemode, gamerule, spectateur, aventure, commandes autorisées)
  - [ ] J12 Orientation et utilitaires
  - [ ] J13 Savoir et journal
  - [ ] J14 Camp de départ
  - [ ] J15 Arbre de progrès
  - [ ] J16 Construction et décoration (familles de blocs, ≥ 200 blocs)
  - [ ] J17 Outils d'équilibrage (crate `balance`)
  - [ ] J18 Clôture

## Défauts signalés

- [x] **Minage sous les pieds** (signalé le 18/11/2026, toujours présent après la v0.2.1) : quand le joueur casse le bloc sur lequel il se tient, il a l'impression d'être téléporté sur le bloc du dessous au lieu de tomber. Cause : le serveur cassait le bloc avant d'appliquer les entrées envoyées avant le clic, et le client rejouait ces entrées sur le monde prédit ; la correction vers le bas s'appliquait d'un coup. Correction (D090) : clics exécutés après les entrées qui les précèdent, rejeu sur le terrain d'avant le clic, petites corrections lissées. Tests `input_order_tests` (serveur, créatif et survie) et `player.rs`.
