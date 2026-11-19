# Améliorations, enclume et marteau de mineur

## À quoi ça sert

Sans classe ni arbre de compétences, le joueur progresse par son équipement. Les **améliorations** rendent un outil, une arme ou une armure meilleurs : plus tranchant, plus solide, plus léger. Elles se posent à l'**enclume**, qui sert aussi à **réparer**. Le **marteau de mineur** est un outil dont la tête s'élargit à l'enclume pour casser plusieurs blocs d'un coup. Tout est décrit dans les packs : un autre pack peut ajouter ses améliorations sans toucher au moteur.

## Comment ça marche

### Les emplacements et leurs familles

Chaque objet a une liste d'**emplacements typés** (jalon 4a) : `upgrade_slots: ["strata:area", "strata:craft"]`. Une famille (`upgrade_slots/<nom>.ron`) dit seulement combien d'emplacements d'elle un objet peut avoir. Les familles existantes : `strata:craft` (artisanal) et `strata:area` (tête du marteau). Les modules mécaniques (v0.8), les runes et les pièces de bâton (v0.9) seront d'autres familles : le moteur ne change pas. Depuis le jalon 7, les modules de four et les améliorations de coffre (familles `strata:furnace` et `strata:storage`) passent par ce même moteur, mais ce sont des **objets posés dans les cases d'un bloc** au lieu d'un enchantement rangé dans la pile : voir [Rangement](rangement.md).

### Une amélioration en pack

```ron
// upgrades/sharpening.ron
Upgrade(
    slot: "strata:craft",
    applies_to: ["strata:tools", "strata:weapons"],
    excludes: [],
    requires: [],
    levels: [
        Level(effects: [
            Stat(stat: "strata:damage", op: Percent, value: 10.0),
            Stat(stat: "strata:mining_speed", op: Percent, value: 10.0),
        ], xp: 2, material: Item("strata:copper_ingot"), count: 2),
    ],
)
```

- `slot` : la famille de l'emplacement qu'elle occupe ;
- `applies_to` : des **tags** d'objets du pack (`strata:tools`, `strata:weapons`, `strata:armour`) ; vide : tout objet qui a un emplacement de la famille ;
- `levels` : chaque niveau **remplace** le précédent ; il dit ses **briques d'effet**, son coût en **XP** et son **matériau** (un objet ou un tag, en nombre) ;
- `requires` / `excludes` : autres améliorations demandées ou interdites.

Les briques : `Stat(stat, op, value)` change une **stat nommée** déclarée dans `item_stats/` (`ItemStat(min, max)` borne les valeurs) ; `op` vaut `Add` (des points) ou `Percent` (× 1 + valeur / 100). Le moteur applique `strata:damage`, `strata:mining_speed`, `strata:durability`, `strata:protection` et `strata:weight` (négatif : plus léger) ; une autre stat déclarée par un pack s'accumule dans `ItemStats::extra`, prête pour le système qui la lira. Seul un *comportement* demande une variante de l'`enum Effect` : `Area(largeur, hauteur)` pour le marteau.

### Où vivent les améliorations d'un objet

Dans les **métadonnées** de la pile (`ItemStack::data`, jalon 1), sous des clés à espace de noms : `strata:upgrades` (liste de paires amélioration, niveau), `strata:repairs` (nombre de réparations), `strata:area` (zone choisie du marteau). Elles se sauvegardent avec la pile, sans rien de plus.

### Un seul calcul des statistiques

`stack_stats(pile, objets, améliorations)` part des valeurs de l'objet (base et tier, déjà résolues au chargement), puis applique les améliorations. **Même effet non cumulable** : si deux améliorations changent la même stat de la même façon, seule la plus forte compte (`strongest`). Pour la main et l'armure, regardées à chaque tick ou image, `StatsCache` garde le résultat tant que l'objet et ses métadonnées ne changent pas. Tout passe par là : temps de casse (serveur et prédiction du client), protection et poids de l'armure portée, utilisations maximales (barre d'usure, infobulle), zones du marteau.

```mermaid
flowchart LR
    I[Objet : base + tier] --> S[stack_stats]
    D[ItemStack.data : améliorations] --> S
    U[upgrades/ en pack] --> S
    S --> T[outil : vitesse]
    S --> W[arme : dégâts]
    S --> A[armure : protection, poids]
    S --> M[utilisations max]
    S --> Z[zones du marteau]
```

### L'enclume

Une station (`station: Some(Anvil)`) à deux cases : l'objet, puis ce qui paie (une pile usée ou améliorée ne sert jamais de matériau). Comme l'établi, ses cases appartiennent au joueur et reviennent à l'inventaire quand l'écran se ferme ou que l'enclume disparaît. Le client calcule la liste des améliorations possibles avec les mêmes packs (montrées par leur matériau ; l'infobulle donne niveau, effets, coût, et pourquoi c'est impossible : plus d'emplacement, prérequis, exclusion). Le serveur reçoit `ChooseUpgrade(Some(index))` et décide ; `None` veut dire « réparer ». `anvil_outcome` calcule le résultat (pur, testé) ; `take_outcome` le donne et retire l'objet et le matériau payé.

**Réparer** : le matériau de l'objet (celui de son tier, `repair` dans `tiers/`, ou le sien, `repair` dans `items/` pour le cuir et le tissu) rend un quart des utilisations par unité ; un second objet identique sans amélioration ajoute ses utilisations et 12 % de plus. Chaque réparation coûte un point d'XP de plus que la précédente. Un objet cassé gardé (option « Objets usés ») se répare comme un autre.

**L'XP** n'existe pas encore : les coûts sont dans les données et affichés, mais `can_pay_xp` répond toujours oui et `pay_xp` ne retire rien (`crates/server/src/game/anvil.rs`). Le jalon 9 n'aura qu'à remplir ces deux fonctions.

### Le marteau de mineur

Un outil du type pioche, plus lent, avec un emplacement de tête. La **tête élargie** a cinq niveaux, de plus en plus chers : 3×1, 5×1, 3×3, 3×5, 5×5. Toutes les zones débloquées restent au choix : la touche « Zone du marteau » (R, clic du stick droit) passe à la suivante (`ChooseArea`), affichée au-dessus de la barre rapide.

À chaque coup, le client envoie la **face** visée (`StartBreak { pos, face }`) ; le serveur la refuse s'il ne peut pas la voir (yeux du mauvais côté, bloc plein devant). Le serveur calcule la zone (`swing_cells`) : en travers de la face, largeur × hauteur sur un mur, largeur × profondeur devant soi sur un sol (selon l'orientation du joueur). Seuls les blocs du type de l'outil qu'il récolte partent ; la casse dure autant que le **bloc le plus dur** de la zone (comme les mods connus) ; chaque bloc (sauf un bloc instantané) use le marteau d'une utilisation, et la zone s'arrête si le marteau casse ; à la fin, la zone est la plus petite entre celle du début et celle choisie, et un bloc plus dur posé entre-temps reste ; les butins tombent en un tas par objet, comme la faucille. Le client fait le même calcul pour prédire le temps et dessine le **contour** de toute la zone autour du bloc visé.

## Dans le code

| Quoi | Où |
|---|---|
| Lecture des packs, briques, statistiques, enclume, zones | `crates/core/src/upgrade/` |
| Textes (effets, coûts, infobulles d'objet amélioré) | `crates/core/src/upgrade/describe.rs` |
| Enclume côté serveur, point d'accroche de l'XP | `crates/server/src/game/anvil.rs` |
| Marteau côté serveur (zone, usure, butin) | `crates/server/src/game/hammer.rs`, `mining.rs` |
| Écran de l'enclume | `crates/client/src/screens/anvil.rs`, `assets/ui/inventory.ron` |
| Zone, contour et touche côté client | `crates/client/src/hammer.rs`, `render/overlays/` |
| Page du wiki | `crates/wikigen/src/upgrades.rs` |
| Contenu | `assets/packs/strata/{upgrades,upgrade_slots,anvil.ron,items,tiers}` |

<details><summary>Côté Rust</summary>

- **`match` sur deux valeurs** : `Effect::same_kind` compare `(self, other)` d'un coup ; deux `Stat` de même stat et même opération, ou deux `Area`, sont de la même sorte. C'est ce qui permet « le plus fort compte ».
- **`RefCell`** : `StatsCache::get` prend `&self` mais modifie son contenu ; `RefCell` vérifie à l'exécution qu'un seul emprunt modifiant existe à la fois. Les résultats sont partagés par `Arc` (un compteur de références) plutôt que copiés.
- **Données extensibles** : `ItemData` est une `BTreeMap<Identifier, DataValue>` ; on lit avec un `match` sur la forme attendue (`DataValue::List(...)`) et on ignore le reste, ce qui rend les vieilles sauvegardes et les données d'autres packs inoffensives.
- **Fonction pure + fermeture** : `swing_cells(centre, face, (zone, lacet), outil, blocs)` reçoit `blocs: impl Fn(BlockPos) -> Option<&Block>` ; le serveur y branche son chargeur de colonnes, le client son monde, les tests une carte en mémoire. Le `'a` dit que les blocs rendus vivent au moins aussi longtemps que l'appel.
- **`Option::is_none_or`** : `choice.is_none_or(offered)` accepte « aucun choix » (réparer) ou un choix proposé, en une ligne.

</details>

## Pour aller plus loin

- Décisions D081 à D085 dans `docs/decisions.md` ; valeurs dans `docs/regles/equilibrage.md` (« Améliorations, enclume et marteau de mineur »).
- [Outils, armes et armures](outils-et-armures.md), [Craft](craft.md), [Récolte](recolte.md).
