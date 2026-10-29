# Vision et décisions de conception

Décisions de design globales de Strata Mundi. Le découpage en versions est dans `docs/roadmap.md`, le détail de chaque version dans `docs/versions/vX.Y.md`. À lire pour toute conception de gameplay ou de contenu.

## Progression du joueur

```
Terra (équipement bois → diamant)
 ├─ Boss de structure (un par type de structure) ─→ pièces de bâton, clés de portail, armes uniques
 ├─ Coffres de structure ─→ sceaux (probabilité de drop) qui débloquent les boss magiques
 ├─ Boss magiques invoqués avec le bâton d'invocation ─→ pièces de bâton, magie avancée
 │
 ├─ Infernum (obligatoire avant le Vacuum) ─→ Ignium (tier après le diamant) + fragment du boss
 ├─ Crepuscula   ┐
 ├─ Abyssus     ├─ ordre libre, chacune avec ses matériaux spécialisés
 ├─ Machina    ┘
 │
 ├─ Vacuum (clé : fragment du boss de l'Infernum + Ignium) ─→ Nihilium (tier après la Ignium)
 │    └─ Dragon (le « boss final ») ─→ faux générique, révélation, tour infinie, zone cachée, Siderium
 │         ├─ Choix : réveiller ou laisser dormir les premiers mortels (rêve de Crepuscula)
 │         └─ Vinculum, le dieu banni (boss secret)
 │
 └─ Caelum (débloqué par la mort de Ianitor)
      ├─ Huit épreuves divines, ordre libre ─→ armes divines, pièces de bâton divines
      ├─ Épreuve d'Arbiter (après les huit) ─→ révélation : les dieux aussi sont l'objet d'un pari
      └─ Choix : accepter, ou se rebeller et tuer les dieux pour de vrai
```

### Deux voies pour chaque dimension

Chaque danger a **une solution magique et une solution mécanique équivalentes**, et les **potions** forment une troisième voie (efficaces mais de durée limitée, produisibles en masse par la mécanique). Le joueur choisit sa voie ou les combine.

| Dimension | Danger | Voie magique | Voie mécanique |
|---|---|---|---|
| Infernum | Chaleur extrême, lave | Sort ou potion de résistance au feu | Armure réfrigérée (circuit de refroidissement) |
| Crepuscula | Brouillard maudit, malédiction | Sorts de vision et de purification | Lanterne purificatrice et masque filtrant |
| Abyssus | Noyade, pression | Sorts de respiration et de peau de pression | Scaphandre mécanique |
| Machina Caeli | Vents violents, chutes | Sorts de lévitation et d'ancrage | Grappin et planeur |
| Vacuum | Le vide | Sorts de lévitation et de téléportation | Propulseur dorsal et grappin |
| Caelum | Selon le royaume | Sorts de l'école opposée au dieu | Équipement mécanique divin adapté |

## Décisions de conception globales

### Durée, rythme et contenu chiffré (détail : `docs/regles/equilibrage.md`)
- ~100 mobs, ~80 sorts, boss principaux et **boss secondaires** (errants, d'événement, de quête, gardiens de gisements).
- Boss exigeants sans excès ; épreuves divines en 3 à 6 essais. Ressources finies ; butin de boss, matériaux magiques et licornes non farmables. La mécanique prend ~1,5× plus de temps que la magie.
- **50 à 80 heures jusqu'à Ianitor**, dont environ **20 heures dans Terra** avant de partir vers les dimensions ; le Caelum et l'après-jeu s'ajoutent.

### Monde
- Hauteur **512 blocs (y 0 à 511)** : surface moyenne ~128, mer à 120, reliefs courants jusqu'à ~256, grands sommets ~400, socle indestructible en bas. Horizontalement infini, bordure réglable (10 millions par défaut).
- **Taille des biomes réglable** (comme Minecraft par défaut) ; **structures plus denses** que Minecraft.
- **Spawn garanti accueillant** : plaines ou forêt, arbres et eau proches, village à moins de ~1 000 blocs.
- Relief mixte selon le biome ; grottes inspirées de *Caves & Cliffs* ; eau et lave fluides.
- Environ **50 biomes** à la v0.13 (hors royaumes divins), biomes souterrains dès la v0.6.
- Structures : **douze types de grandes structures à boss** (donjon, tour de mage, ruine engloutie, atelier, temples du désert et de la jungle, labyrinthe du minotaure, citadelle des glaces, mine, sanctuaire de la civilisation endormie, bastion gobelin, cathédrale engloutie), **villages fortifiés de fanatiques** (rares, hostiles, à libérer par le combat ou la diplomatie), villages, tavernes, donjons, mines, temples, tours, ruines englouties, ateliers.
- **Temps** : journée de 20 minutes (10 de jour, 10 de nuit au départ) dont la répartition varie avec la saison (plus de jour en été, plus de nuit en hiver). **4 saisons de 28 jours.**
- **Nuits plus dangereuses** que Minecraft : plus sombres loin des lumières, plus de monstres.
- **Météo aléatoire** comme Minecraft, mais influencée par la saison et le biome.
- **Événements** : fêtes de village saisonnières, événements célestes (lune de sang, étoiles filantes, éclipse, phases de lune) avec effets de jeu, migrations et floraisons.
- **Intégrité structurelle** (désactivable) : chaque matériau a une portée en surplomb ; au-delà, les blocs tombent et restent au sol. S'applique aux constructions comme aux mines.
- **Feu** qui se propage, désactivable.
- **Minerais** : petits filons partout et grands gisements riches et rares.

### Dimensions
- **Terra**, **Infernum**, **Crepuscula**, **Abyssus**, **Machina Caeli**, **Vacuum**, **Caelum** (royaumes divins séparés, inspiration gréco-latine façon mod Aether).
- **Chaque monde est l'œuvre d'un dieu** : Terra (Saxa et Sanatrix), Infernum (Fornax), Crepuscula (Somnifer), Abyssus (Marinus), Machina Caeli (Ventus et Fulgur), Vacuum (Hiemalis), Caelum (Arbiter). Indices du créateur, peuples dévots, sorts de son école renforcés dans son monde.
- Chaque dimension a ses **peuples** (Infernum : forgerons de cendre, âmes errantes, démons mercantiles ; Crepuscula : fées et lutins, bergers du crépuscule, esprits des arbres ; Abyssus : peuple-sirène, descendants des noyés, créatures-coquillages ; Machina Caeli : automates conscients, ingénieurs exilés ; Caelum : peuple divin), **sauf le Vacuum, volontairement vide**. Ordre libre, sauf l'Infernum obligatoire avant le Vacuum, et Ianitor avant le Caelum.

### Survie
- Vie, faim, soif, température, endurance, expérience. Options à la carte à la création du monde.
- Mort façon Minecraft ; option pour l'inventaire à la mort dans une dimension (reste sur place ou tombe protégée).
- Craft par grille + livre de recettes.
- Tiers linéaires bois → pierre → fer → diamant → **Ignium** → **Nihilium** → **Siderium** → équipement divin. Matériaux **spécialisés** dans les autres dimensions. Noms provisoires.
- **Aucune classe, aucun arbre de compétences** : la façon de jouer dépend **uniquement de l'équipement**.
- Tout est améliorable : emplacements d'amélioration, modules mécaniques, runes, pièces de bâton, accessoires d'armes, équipement de familier.
- Arbre de progrès façon advancements.

### Combat
- **Dégâts physiques + huit éléments** (un par école), résistances et faiblesses par mob ; effets de statut classiques, de contrôle, positifs et malédictions ; mobs à force fixe, plus forts dans leur environnement naturel.
- Ressenti **sobre comme Minecraft**. Mobilité de base façon Minecraft, enrichie par l'équipement (double saut, dash, escalade).
- Familles d'armes : **épées**, **armes légères** (dagues doubles, rapière, fouet), **armes lourdes** (marteau, masse, hache à deux mains), **armes d'hast** (lance, hallebarde, faux), **arcs et arbalètes**, **armes de lancer** (couteaux, javelots, boomerang, bombes), **magie** (baguettes, bâton de sorts), **armes à feu**.
- Chaque famille a un style distinct (portée, vitesse, combos, coût d'endurance), inspiré des façons de combattre de D&D, mais sans classe.
- Combat action dès la v0.6 : esquive, parade, combos, attaques chargées.
- **Armes à feu en progression** : poudre noire d'abord (pistolet à silex, mousquet, tromblon, tir unique, rechargement manuel), puis armes mécaniques avancées (répétition, air comprimé, rotatives). Munitions fabriquées ou produites en machine, surchauffe et enrayement pour les armes avancées, accessoires modulaires (lunette, crosse, canon). Seuls les **explosifs** cassent les blocs.
- Magie et mécanique sont **équivalentes** : on peut finir le jeu en mage, en ingénieur-tireur, ou en mélangeant.

### Magie
- Baguettes en début de jeu ; **bâton de sorts** en trois pièces (manche = mana, cœur = puissance, pommeau = sorts disponibles) ; **bâton d'invocation** pour les boss magiques et les montures magiques, amélioré à chaque victoire, avec retour possible aux boss vaincus.
- Huit écoles : feu, glace, foudre, terre, air, vie, eau, esprit — chacune liée à un dieu du Caelum.
- Sorts inspirés de D&D pour leur variété (contrôle, invocation, protection, utilitaire, dégâts).

### Bestiaire
- Animaux, monstres, boss, villageois.
- **Créatures mythologiques et de fantasy libres de droits** (inspiration D&D) : gobelins, kobolds, trolls, ogres, harpies, minotaure, basilic, hydre, cockatrice, mimic, liche, golems, gargouilles, élémentaires, satyres, centaures, griffons, cyclopes… Pas de créature propre à une marque.

### Villages et tavernes
- Commerce en **pièces frappées** (1 or = 20 argent = 240 cuivre), frappables avec garde-fous contre l'inflation ; quelques recettes et objets exclusifs aux marchands. PNJ nommés (~800 noms par peuple) avec traits de personnalité ; villages qui grandissent ou déclinent ; **villages fondés et gérés par le joueur** (mini city-builder : besoins, prospérité, métiers) ; parcelles à acheter (réductions et quêtes exclusives pour les résidents). Pas d'autre voyage rapide que les balises.
- Métiers, quêtes, défense du village.
- **Tavernes façon D&D** : PNJ hauts en couleur, tableau de quêtes d'aventure, rumeurs menant à des donjons, jeux de hasard (dés, cartes, coffres mystère, paris sur combats d'arène).

### Objets et équipement
- **Armures légère / intermédiaire / lourde** : la lourde protège plus mais réduit vitesse, endurance et mana.
- **Accessoires** à emplacements fixes : 2 anneaux, 1 amulette, 1 ceinture, 1 cape (ou ailes) ; bottes spéciales dans l'emplacement des bottes. **Bonus de set** pour les pièces assorties, sets artisanaux (par matériau) et sets de boss ou de dieux.
- **Uniques nommés** pour les objets exceptionnels, aucune rareté aléatoire.
- **Durabilité** réglable par option de monde : l'objet casse à zéro, ou devient inutilisable jusqu'à réparation.
- **Rangement** : sac à dos améliorable façon mod *Sophisticated Backpacks* (tiers, améliorations : ramassage, filtre, aimant, nourrissage, établi intégré, réservoir…), bourses et sacoches spécialisées (graines, munitions, potions), coffre lié accessible partout, stockage en réseau (terminal mécanique ou réseau magique).
- **Outils** : pioche, hache, pelle, houe, outils spécialisés (cisailles, faucille, burin), outil multi-usage, outils de construction (baguette de bâtisseur, symétrie), outils de fouille, **marteau de mineur** dont la zone (1×1, 3×1, 5×1, 3×3, 3×5, 5×5) se débloque par améliorations de plus en plus chères, avec retour possible aux zones plus petites.
- **Utilitaires** : cordes et échelles portables, lanternes portables (lumière dynamique), tentes et sacs de couchage, longue-vue, détecteurs de minerais et de structures.
- **Consommables** : nourriture, potions, parchemins de sort à usage unique, bandages et trousses de soin, huiles d'arme (feu, poison, sacré), pierres de retour.
- **Serrures et clés** : portes et coffres verrouillés ; ouverture par **crochetage** (mini-jeu façon Skyrim, à la souris ou aux sticks), par **sort d'ouverture** ou par **perceuse à serrure** mécanique.
- **Stations de travail spécialisées** : forge, enclume, tannerie, métier à tisser, joaillier, armurier, alchimie, cuisine, tailleur de pierre, établi d'assemblage du bâton, etc.
- **Cosmétiques** : apparence d'armure, teintures d'équipement, chapeaux et vêtements décoratifs.
- **Loisirs** : instruments jouables, disques et boîtes à musique (musiques procédurales à collectionner), peintures et bannières à motifs, jeux de société en bloc.
- **Livres** : livres écrits par le joueur, livres de lore trouvables, journal automatique. Le lore sera écrit après la v1.0.

### Inventaire
- Comme Minecraft (27 + 9 cases, armure, main secondaire), avec en plus les accessoires, le sac à dos et les sacoches.

### Familiers
- **Tous les familiers servent au combat** : attaquants (chien, loup, chat sauvage), soigneurs (fée, perroquet, petit esprit), protecteurs (tortue géante, golem miniature), utilitaires (ramassent le butin, détectent les pièges, éclairent).
- **2-3 familiers actifs** à la fois ; ils progressent **uniquement par leur équipement** (armure, collier, harnais).

### Savoir et découverte (jouable sans wiki)
- **Recettes** : les basiques sont connues dès le départ (planches, bâtons, établi, outils en bois et pierre, torches). Les autres s'apprennent par des **pages de recettes** trouvées dans les structures, achetées ou enseignées par les PNJ, découvertes par **expérimentation** dans la grille, ou par **analyse** d'un objet trouvé. Disposer correctement une recette sans la page fonctionne quand même (pour les joueurs qui connaissent déjà) et ajoute la page au journal. Le livre de recettes remplit la grille pour les recettes apprises.
- **Usage** : les statistiques chiffrées, les effets cachés et les bonus de set se révèlent en utilisant ou en portant l'objet. Les faiblesses des mobs se révèlent en les frappant avec le bon type de dégâts.
- **Sorts** : un bouton de **lancer à l'aveugle** déclenche un sort au hasard parmi ceux que le bâton permet mais que le joueur n'a pas encore découverts ; le sort est alors appris et noté au journal.
- **Journaux abandonnés** dans le monde : pages de recettes, informations sur objets, sorts, mobs et boss, indices de lieux (structures, licornes, secrets), fragments de lore.
- **Journal du joueur** : recettes, objets étudiés, sorts, bestiaire, lieux, quêtes. Il suffit, avec l'arbre de progrès et les PNJ, pour finir le jeu sans wiki.
- Infobulles simples, enrichies au fil des découvertes. En créatif, tout est connu.

### Licornes
- **Extrêmement rares**, dans plusieurs biomes et dimensions.
- **Montures** apprivoisables ; sur une licorne vivante, le joueur peut **lancer tous les sorts du jeu grâce à sa corne**.
- La **corne de licorne** est la **meilleure pièce de bâton du jeu** : elle remplace à elle seule manche, cœur et pommeau et donne accès à tous les sorts (à découvrir comme les autres). Trois façons de l'obtenir : tuer une licorne (toutes les autres licornes fuiront ensuite le joueur à jamais), qu'une licorne apprivoisée très âgée la perde (très rare), ou trouver des **fragments de corne** en donjon (version moins puissante).

### Têtes de mobs
- Chaque mob (boss et dieux compris, plus rarement) peut lâcher sa tête, façon mod *MobHeads* : faible chance de base, augmentée par une arme de bourreau, une rune ou un module de décapitation.
- Posables en décoration, portables en casque cosmétique (certaines trompent la détection de leur espèce), collection suivie dans le journal et les progrès.

### Monde, départ et outils de créateur
- Types de monde : normal, **plat personnalisable**, **amplifié**, **biome unique / îles**, et **éditeur de génération** (taille des biomes, fréquence des structures, minerais…).
- Le joueur se réveille dans un **petit camp** (feu de camp, tente comme premier point de réapparition, coffre avec quelques affaires basiques, journal vierge, indice vers le premier village).
- **Mode photo**, **plans de construction** (copier une construction et la reconstruire ailleurs), **outils de terraformation** créatifs.
- HUD : chiffres de dégâts et barres de vie des mobs (désactivables), marqueur de quête optionnel.
- **Ambiance chaleureuse et merveilleuse**, humour léger, objets et lieux secrets, clins d'œil, seeds spéciales.

### Lore
- Découverte réglable à la carte (recettes, statistiques, sorts, bestiaire, lore) pour les joueurs qui préfèrent tout connaître.
- Bases validées dans `docs/lore/fondations.md` (séances 1 et 2) : Ianitor était **Viator**, un champion que Hiemalis a sauvé en le changeant en gardien ; les **premiers mortels** dorment dans le rêve de Crepuscula, protégés par Somnifer ; les fanatiques servent sans le savoir **Vinculum**, dieu banni des chaînes ; le joueur n'est personne ; les dieux, retirés derrière un dragon gardien, parient sur les mortels ; après le dragon, les légendes deviennent vraies.
- Écrit **ensemble** avec l'utilisateur, en séances dédiées ; en attendant, emplacements balisés `[LORE À ÉCRIRE]` et listés dans `docs/lore/emplacements.md`.

### Rôle des deux voies
- **Mécanique** : pour ceux qui aiment automatiser et farmer ; plus longue, récompense les usines.
- **Magie** : pour ceux qui préfèrent explorer et combattre ; ressources rares (butin, boss, licornes), **aucune automatisation**.

### Mécanique (inspirée de Create, noms et textures originaux)
- Énergie rotative avec charge, machines, logistique, circuits intégrés, transport mécanique, armes à feu avancées, explosifs, équipements de dimension.

### Exploration et transport
- Bateaux, chevaux, montures magiques, transport mécanique.
- Cartes à dessiner, minimap optionnelle, boussole et coordonnées, balises de téléportation façon Waystones.

### Activités
- Pêche, cuisine avancée, apiculture et élevage, archéologie, jeux de hasard ; pêche complète en v1.2 et activités des versions v1.2 à v1.8.

### Présentation
- Pixel art 16×16 généré par code, style inspiré de Minecraft et des mods cités, textures originales ; **packs de ressources** 16-64 px simples et documentés, textures animées, connectées et variantes.
- Énormément de blocs de construction et de déco.
- Vue 1re et 3e personne, skin personnalisable, armure et armes visibles.
- Écran titre avec panorama ; liste des mondes avec icônes ; un personnage par monde.
- HUD : vie, endurance et mana en haut à gauche façon Dark Souls ; faim, soif et température au-dessus de la hotbar ; XP façon Minecraft ; nom du bloc visé façon Jade ; bande de cap façon Skyrim ; tout repositionnable.
- Dialogues à choix avec conséquences (réputation, embranchements de quêtes).
- Interface pixel art, sons synthétisés, langues **français, anglais et LOLcat**.
- Sons synthétisés au rendu réaliste (repli CC0 possible avec accord) ; musique procédurale pré-rendue, calme et rare façon C418, adaptative en combat, avec un thème par dimension, par dieu et par boss majeur ; réverbération et occlusion (stéréo simple sur DS) ; petits sons pour les PNJ (pas de voix).
- Messages de mort humoristiques, écran de statistiques façon Minecraft (plus complet en hardcore), récapitulatif après chaque boss.
- **Réputation par peuple**, sans alignement moral global.
- Difficulté fixe (options du monde et zone), pas de difficulté qui monte avec le temps.

### Performance et plateformes
- Windows et Linux, dont la **console DS** de l'utilisateur (Intel N100 à 8-10 W, Linux maison, deux écrans) : **30 FPS stables** avec un préréglage dédié.
- **Tout s'active, se désactive ou se règle**, comme dans Minecraft et au-delà.
- Clavier/souris et **manette dès la v0.1**.

### Wiki
- Site statique HTML/CSS/JS vanilla, inspiré du wiki Minecraft, en **français, anglais et LOLcat**, régénéré à chaque version par la CI et publié sur la branche `wiki`, données tirées des packs.

### Modding
- Le contenu de base est un pack comme un autre, surchargeable.
- **v1.1** : dossier `mods/` à glisser-déposer avec menu d'activation, scripts **Lua**, éditeur en jeu (blocs, objets, recettes, textures pixel art, mobs simples, export), mod d'exemple et guide.

### Technique
- Solo jusqu'à la v0.13, architecture client/serveur prête dès la v0.1 ; **multijoueur en v1.0**.
- Rust stable, moteur maison sur `wgpu` en **Vulkan uniquement**, **ECS maison**, simulation à 20 TPS.
- Bordure de monde réglable (10 millions de blocs par défaut).
- Config en TOML, packs en RON.
- **Dépôt public** : code MIT, contenu CC BY-NC 4.0, issues ouvertes, tout sur `main`.
- Distribution : `.zip`, `.tar.gz`, installeur `.msi`, AppImage, Flatpak.
- Vérification de mise à jour au lancement, rapports de plantage avec issue pré-remplie, aucune télémétrie, copies de sécurité des mondes.
- Tests ciblés + benchmarks en CI, profilage Tracy.
- Sauvegardes compatibles garanties à partir de la v1.0.
