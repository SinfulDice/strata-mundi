# Réglages

## À quoi ça sert

Adapter le jeu à la machine (du PC puissant au DS à 8 W), au joueur (sensibilité, touches, police lisible, filtres daltoniens) et à ses habitudes (sauvegardes, packs de ressources). Règle du projet : **tout s'active, se désactive ou se règle**, et chaque réglage coûteux peut être baissé.

## Comment ça marche

### Un seul fichier

Tout est dans `config/settings.toml` (dans le dossier de données), en quatre parties :

```toml
preset = "ds"            # absent = « personnalisé »

[graphics]               # distance, échelle de rendu, images/s, VSync, FOV, AO, brouillard…
[interface]              # langue, taille, police, filtre daltonien, effets caméra, HUD
[controls]               # sensibilités, axe inversé, zone morte, saut auto, touches
[game]                   # mises à jour, sauvegarde auto, copies, arrière-plan, packs
```

Une valeur absente ou invalide reprend sa valeur par défaut, sans toucher au reste : un fichier abîmé ne casse jamais le jeu. Les touches sont écrites par action (`MoveForward = [{ Key = "KeyW" }, { Pad = "DPadUp" }]`) pour rester modifiables à la main. `KeyW` est la **position** de la touche (Z sur un clavier AZERTY).

### Préréglages

Très bas, bas, moyen, haut, ultra et **DS** remplissent toute la partie `graphics` d'un coup. Toucher ensuite un seul réglage graphique fait passer en « personnalisé ». `--preset <nom>` en ligne de commande applique un préréglage pour ce lancement sans l'écrire.

### Application à chaud

L'écran d'options modifie une **copie** des réglages. À chaque image, la session compare cette copie à celle en vigueur et ne refait que ce qui a changé :

| Changement | Ce qui est refait |
|---|---|
| Distance de rendu | message `ViewDistance` au serveur |
| Occlusion ambiante, fils de maillage | tout le monde est re-maillé (l'ancien maillage reste affiché en attendant) |
| Mipmaps, anisotropie | le tableau de textures et son échantillonneur |
| VSync | la surface de la fenêtre |
| Échelle de rendu, filtre daltonien, brouillard | rien à reconstruire : des valeurs passées aux shaders |
| Touches, sensibilités, zone morte | l'état des entrées |
| Saut auto, montée des demi-blocs | message `MovementPreferences` au serveur |
| Packs de ressources | rechargement des ressources (comme F5) |

Le fichier est écrit quand un changement est fini (pas à chaque image d'un glissement de curseur).

### Les effets de rendu

- **Échelle de rendu** : la 3D est dessinée dans une image plus petite (ou plus grande) que la fenêtre, puis étirée par une **passe finale** ; l'interface reste nette, dessinée par-dessus à pleine résolution. À 75 %, la carte graphique a 44 % de pixels en moins à calculer.
- **Filtres daltoniens** (protanopie, deutéranopie, tritanopie) : dans la même passe finale. On simule ce que l'œil ne distingue pas, puis on reporte cet écart vers des couleurs qu'il distingue (méthode « daltonize »). Sans échelle ni filtre, la passe finale n'existe pas : coût nul.
- **Brouillard** : dans le shader des blocs, le lointain se fond dans la couleur du ciel, ce qui cache la limite du monde chargé.
- **Mipmaps** : des copies réduites de chaque texture pour les blocs lointains (moins de scintillement). **Anisotropie** : sols nets vus de biais, au prix de pixels adoucis de près.
- **Limite d'images/s** et **arrière-plan** : la boucle attend avant l'image suivante (`ControlFlow::WaitUntil`) ; fenêtre inactive = 5 images/s, ce qui économise la batterie du DS.

### Accessibilité

- **Police lisible** : DejaVu Sans, rastérisée au chargement à la taille réelle de l'écran (×2, ×3… selon l'échelle d'interface) pour rester lisse. Remplaçable par un pack de ressources (`fonts/readable.ttf` + `readable.ron`).
- **Taille de l'interface** : auto (le plus grand multiple entier qui tient) ou ×1 à ×6.
- **Réduire les effets caméra** : pas d'élargissement du champ de vision au sprint.

### Packs de ressources

L'onglet **Packs** liste les dossiers de `resourcepacks/` qui contiennent un `pack.ron`. Valider active ou désactive un pack ; gauche/droite change sa priorité (n° 1 = gagne sur les autres). Le changement recharge aussitôt textures, traductions et interface.

## Où c'est dans le code

- `crates/client/src/settings/` : `Settings` et ses quatre parties, chargement et écriture, préréglages.
- `crates/client/src/screens/options/` : l'écran (onglets, lignes, valeurs, pas de chaque réglage, packs).
- `crates/client/src/session/apply.rs` : comparaison des réglages et application.
- `crates/client/src/render/post.rs` et `shaders/post.wgsl` : passe finale (échelle, filtres).
- `crates/core/src/font/truetype.rs` : police TrueType rastérisée avec `fontdue`.

<details><summary>Côté Rust</summary>

- **`#[serde(default)]`** sur une structure : chaque champ absent du fichier prend la valeur de `Default::default()`. C'est ce qui rend le fichier tolérant aux versions plus anciennes.
- **`#[serde(from = "…", into = "…")]`** : les touches sont gardées en mémoire comme une liste de paires, mais écrites dans le fichier sous une autre forme (une table par action) ; serde passe par le type intermédiaire.
- **`PartialEq` sur `Settings`** : comparer deux réglages entiers d'un `==` permet de savoir en une ligne s'il y a quelque chose à appliquer.

</details>
