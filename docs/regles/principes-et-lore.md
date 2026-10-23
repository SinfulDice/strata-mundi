# Principes de conception et lore (version détaillée)

- Durée visée : **50-80 h jusqu'à Ianitor**, ~20 h dans Terra. Équilibrer les coûts et le butin en conséquence.

## Principes de conception

- **Le jeu doit pouvoir se finir sans le wiki.** Tout ce qu'il faut savoir est découvrable en jeu : arbre de progrès qui guide, livre de recettes, infobulles qui indiquent les utilisations d'un objet, journal automatique (découvertes, créatures, lieux), indices des PNJ et de l'oracle, avertissements clairs avant un danger. Le wiki approfondit, il ne remplace pas.
- **Deux voies, magie ou mécanique**, pour chaque obstacle majeur (dimensions, serrures, déplacement…). Elles ne se jouent pas pareil :
  - **Mécanique** = pour ceux qui aiment **automatiser et farmer** ; voie plus longue, qui récompense l'investissement en usines.
  - **Magie** = pour ceux qui préfèrent l'**exploration et le combat** ; ses ressources viennent du butin, des boss, des créatures rares. **Aucune automatisation magique** (exceptions assumées : familiers qui ramassent le butin, coffre lié).
- **Pas de classe ni d'arbre de compétences** : la manière de jouer vient de l'équipement.
- **Exception** : l'enchantement façon Minecraft (v0.9) reste un tirage aléatoire, visible et choisi, en plus des runes.
- **Les potions sont une science** (alchimie expérimentale), automatisables par la mécanique ; elles forment une troisième voie, limitée dans le temps, pour survivre aux dimensions.
- **Effets non cumulables** : potions, sorts et plats peuvent se combiner, mais un même effet ne se cumule pas (seul le plus fort compte).
- **Pas de raretés aléatoires** (pas de commun → légendaire à propriétés tirées au sort) : le butin courant est fixe, les objets exceptionnels sont des **uniques nommés**.
- **Ambiance chaleureuse et merveilleuse** (danger réel mais jamais sombre gratuitement), **humour léger** (descriptions, PNJ farfelus, clins d'œil), secrets et easter eggs.
- **Le savoir est une ressource.** On ne sait pas tout d'emblée : les recettes s'apprennent, les statistiques d'un objet se révèlent en l'utilisant, les sorts se découvrent en les lançant, les faiblesses des mobs en les frappant. Le **journal** centralise tout ce que le joueur a appris. En créatif, tout est connu. En survie, la découverte se règle **à la carte** dans les options du monde (recettes, statistiques d'objets, sorts, bestiaire, lore) pour les joueurs aguerris ou ceux qui n'aiment pas cette partie.

## Lore

- **Tu n'écris jamais de lore toi-même.** Le lore est conçu avec l'utilisateur, en séances dédiées : idées proposées de part et d'autre, choix de la façon dont chaque élément se découvre, rédaction validée ensemble.
- Partout où du lore est prévu (livres, journaux abandonnés, journal du joueur, dialogues, descriptions de boss et de dieux, noms de lieux), pose un texte provisoire balisé `[LORE À ÉCRIRE]` et ajoute une entrée dans **`docs/lore/emplacements.md`** (où, déclencheur de découverte, longueur visée, contraintes).
- Les décisions de lore validées vont dans `docs/lore/` (un fichier par thème) et deviennent la référence. Commence par lire `docs/lore/fondations.md`.
- **Tous les noms propres sont en latin**, choisis selon ce que la chose fait (voir `docs/lore/fondations.md`). Pour un nouveau nom provisoire, propose un nom latin et marque-le `[NOM À VALIDER]`.
