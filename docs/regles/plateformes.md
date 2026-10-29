# Plateformes, réglages et contrôles

## Matériel cible

| Profil | Matériel | Objectif |
|---|---|---|
| PC de jeu | carte graphique dédiée | 144 FPS, distance 16+, tous effets |
| Portable | iGPU récent | 60 FPS, distance 8, effets lourds coupés |
| **DS** | Intel N100 (iGPU UHD 24 EU), 8 Go RAM, limité à **8-10 W**, Linux From Scratch, deux écrans (haut AMOLED ~5,5", bas tactile résistif), résolution non encore choisie | **30 FPS stables**, plutôt joli que rapide |

- Le profil **DS** est un préréglage à part entière, testé à chaque version : rendu à échelle réduite puis agrandi, distance modérée, effets légers conservés (AO, brouillard), ombres et reflets coupés ou au minimum.
- Le DS tourne sous un Linux maison : le rendu doit passer par **Vulkan (Mesa, pilote Intel)**, sans dépendre d'un environnement de bureau. Documente dans `docs/explications/` les bibliothèques système nécessaires (Vulkan, audio, entrées) pour que l'utilisateur les installe sur sa distribution.
- Le jeu doit supporter **plusieurs écrans** dans son architecture (le contenu de l'écran du bas sera décidé plus tard) et la **saisie tactile** comme source d'entrée possible. Ne pas implémenter l'interface du second écran avant que l'utilisateur ne la définisse.
- Consommation : ne jamais tourner plus vite que nécessaire (limite de FPS, pause du rendu en arrière-plan, veille propre quand la fenêtre perd le focus).
- Overlay F3 : FPS, temps de frame (CPU/GPU), position, chunk, biome, dimension, chunks chargés, files de travail, mémoire.
- Mesure avant d'optimiser ; consigne les mesures de référence dans `PROGRESS.md`, par profil.

## Réglages

Comme Minecraft, et plus : **tout s'active, se désactive ou se règle**, dans le menu et dans le fichier de config.
- **Préréglages** : très bas, bas, moyen, haut, ultra, **DS**. Modifier un réglage passe en « personnalisé ».
- **Graphismes** : distance de rendu, **échelle de rendu** (50-200 %), limite de FPS, VSync, FOV, AO, lissage de la lumière, nuages (désactivés/rapides/détaillés), particules (toutes/réduites/minimales), feuillage détaillé ou opaque, mipmaps, anisotropie, brouillard, ciel et étoiles, distance d'affichage des entités, animations des textures, météo visuelle, ombres (qualité), reflets (qualité), balancement de la vue, effets de distorsion, luminosité.
- **Simulation** : distance de simulation, nombre maximal de mobs, fréquence de mise à jour des fluides, nombre de threads de génération.
- **Interface** : échelle, opacité du HUD, minimap, coordonnées, sous-titres, langue.
- **Contrôles** : clavier/souris et manette remappables, sensibilités, inversion, zone morte.
- **Audio** : volume par catégorie.

## Contrôles

- **Clavier/souris et manette dès la v0.1.** Toute l'interface (menus, inventaire, craft, console exceptée) doit être utilisable entièrement à la manette.
- Manette : disposition façon New 3DS (croix, ABXY, L1/R1, gâchettes analogiques L2/R2, deux sticks, Start/Select, deux boutons arrière). Les gâchettes analogiques et les boutons arrière doivent être assignables.
- Clavier : **codes physiques** (`winit::keyboard::KeyCode`) : WASD physique = ZQSD sur AZERTY. Le nom des touches affiché reflète la disposition réelle.
- Les invites à l'écran s'adaptent au dernier périphérique utilisé (touche clavier ou bouton de manette).
- **Navigation manette façon Minecraft Bedrock** : focus sur les cases et boutons à la croix ou au stick, gâchettes pour changer d'onglet, boutons dédiés (prendre la moitié d'une pile, transfert rapide, déposer, trier). **Curseur virtuel** au stick en option.
- Console de commandes (touche T ou `/`) dès la v0.1.

## Affichage et interface

- **Échelle d'interface entière** façon Minecraft (×1, ×2, ×3…) pour un pixel art net ; échelle « auto » qui choisit la plus grande valeur qui tient à l'écran.
- **Tout ratio d'écran** (16:9, 16:10, 4:3, 21:9…) : l'interface s'ancre aux bords et au centre, rien n'est pensé pour un seul format.
- **Fenêtré** et **plein écran fenêtré** (sans bordure) uniquement ; pas de plein écran exclusif.
- **Tactile** : architecture prête, mais l'utilisation au stylet sera définie avec l'écran du bas du DS ; ne rien implémenter avant.

## Accessibilité

- **Dès la v0.1, tout ce qui est simple** : police lisible en option, taille du texte, modes daltoniens (filtres de couleur et palettes d'interface adaptées), réduction des effets de caméra, sensibilités séparées ; **sous-titres** dès qu'il y a du son (v0.2).
- Le reste (sous-titres directionnels, aide à la visée manette, tutoriel) arrive au fil des versions et est complété en v0.13.

