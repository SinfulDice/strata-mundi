# Futures versions — après le multijoueur

**Statut : À valider.** Idées retenues par l'utilisateur le 24/10/2026. Toutes passent **après le multijoueur (v1.0)**, dans cet ordre (renumérotées le 29/10/2026) :

| Version | Nom provisoire | Pourquoi cet ordre |
|---|---|---|
| v1.4 | Villes et arènes | Base des sièges, des guildes et des arènes |
| v1.5 | Sièges | S'appuie sur les villes et les villages |
| v1.6 | Guildes | Sièges de guilde dans les villes |
| v1.7 | Mers et pirates | Pirates qui assiègent les côtes, navires = structures mobiles |
| v1.8 | Catastrophes | Touche tous les systèmes précédents |

Chacune a son fichier `docs/versions/vX.Y.md`, à détailler avant de commencer.

## v1.4 — Villes et arènes

### Villes et villages

- Trois tailles : **hameau → village → ville**, voire capitale par peuple.
- **Quelques villes générées dès la création du monde** (rares), en plus des villages qui peuvent grandir jusqu'à devenir des villes.
- Une agglomération **grandit ou décline** selon sa prospérité (lien avec la v0.7 et les villages gérés par le joueur).
- Chaque taille débloque des bâtiments : taverne au village, marché, banque, sièges de guilde et **arène** en ville.
- Les villes ont des quartiers, des remparts, une garde et plus de PNJ nommés.
- **Lois locales (décidé)** : chaque ville a ses propres lois et sanctions en cas de crime (vol, meurtre…) : poursuite par la garde, amende, prison ou bannissement selon la ville.
- **Ville du joueur (décidé)** : un village géré par le joueur peut grandir jusqu'à devenir une ville, avec arène et sièges de guilde ; le joueur y fixe ses propres lois.
- Point technique : PNJ nombreux en ville, donc niveaux de détail d'IA indispensables en profil DS.

### Arènes

- Présentes dans les villes ; arènes réduites possibles dans les gros villages.
- **Tournois** de combat (un contre un, par équipes avec familiers, contre des créatures), **joutes** à cheval, **courses** de montures et de véhicules.
- **Créatures d'arène (décidé)** : l'arène a son propre bestiaire, et le joueur peut aussi inscrire des créatures qu'il a capturées (voir ci-dessous).
- **Mort** : les combats normaux se terminent par une défaite sans mort (KO, inventaire conservé) ; les **combats à mort** sont réservés à un mode défi, pour les gros enjeux.
- Classement par ville et classement général, titres et récompenses uniques.
- Paris sur les combats, lien avec les jeux de hasard des tavernes.
- Règles par tournoi (armes autorisées, sans magie, sans armes à feu…), pour mettre en valeur chaque famille d'armes.

### Capture et ménageries (décidé)

- **Outils** : filets et pièges (avec appât), cages à poser ou à transporter (taille selon la créature), **sort de capture** côté magie, **machines de capture** côté mécanique (pistolet à filet, cage automatique).
- **Condition** : la créature doit être affaiblie ; plus sa vie est basse, plus la capture réussit.
- **Cibles** : monstres et animaux. Jamais les boss, les licornes ni les PNJ.
- **Usages** :
  - l'inscrire en arène ;
  - la vendre (arène, collectionneurs, PNJ) ;
  - l'**exposer** dans une ménagerie ou un zoo ;
  - l'**étudier**, ce qui révèle ses faiblesses et statistiques au journal (lien avec « le savoir est une ressource ») ;
  - la relâcher (ailleurs, ou pour gagner de la réputation).
- **Ménageries** : chaque créature demande sa nourriture et un enclos adapté, sinon elle dépérit ou s'échappe. Un zoo dans une ville attire des visiteurs et augmente sa prospérité.
- **Familiers** : certains monstres capturés peuvent devenir des familiers, en plus de ceux déjà prévus. Ils progressent uniquement par leur équipement, comme les autres, dans la limite de 2-3 familiers actifs.
- Point d'équilibrage : une créature capturée ne doit pas devenir une source de butin farmable si son espèce lâche des ressources non farmables.
- Point technique : créatures en enclos simulées en IA simplifiée, et gelées hors distance de simulation comme le reste.

## v1.5 — Sièges façon tower defense

- Les raids deviennent de **vrais sièges** par vagues, annoncés à l'avance : le joueur a le temps de préparer sa défense. **Délai de préavis réglable** dans les options du monde.
- **Récompenses (décidé)** : réputation auprès du peuple défendu, butin des assaillants, objets uniques nommés pour les grands sièges, prospérité accrue du village après une victoire.
- **Assaillants** : fanatiques, hordes de monstres, pirates (villages côtiers, à partir de la v1.7).
- **Cibles** : les villages et villes, et **la base du joueur** quand elle attire l'attention (taille, richesse, réputation).
- **Défenses à placer** : murs, portes renforcées, pièges, tourelles mécaniques, balistes, catapultes, totems ou glyphes magiques.
- Ennemis variés par vague : fantassins, béliers, tours d'assaut, sapeurs, créatures volantes.
- Les PNJ participent selon leurs métiers (gardes, archers, forgerons qui réparent).
- Lien avec l'intégrité structurelle : les murs peuvent céder.
- Réglable : fréquence, difficulté, sièges de la base du joueur, désactivation.

## v1.6 — Guildes

- Guildes à rejoindre : marchands, aventuriers, mages, ingénieurs, pêcheurs, chasseurs…
- **Appartenance à toutes les guildes en même temps** possible, chacune avec ses rangs.
- **Rangs** gagnés par des contrats de guilde ; avantages concrets (recettes exclusives, prix, accès à des lieux).
- **Guilde fondée par le joueur** :
  - recruter des PNJ qui partent en expédition et rapportent des ressources ; ils ne meurent pas, mais peuvent revenir **blessés** et doivent se reposer (mieux équipés, ils se blessent moins) ;
  - construire un siège qui débloque des services ;
  - rivaliser avec les autres guildes (classements, concurrence) ;
  - blason (lien avec les bannières), nom, tenues.
- Pas de classe ni d'arbre de compétences : les avantages passent par l'équipement, le savoir et l'accès.
- Point d'équilibrage : les expéditions des PNJ ne doivent jamais rapporter de ressources non farmables (butin de boss, matériaux magiques, licornes).

## v1.7 — Mers et pirates

- Navires construits **bloc par bloc**, puis animés depuis un gouvernail (structures mobiles, déjà listées dans les idées de la roadmap).
- **Combat naval** : canons, abordages, navires ennemis.
- **Peuple pirate** avec sa réputation : ennemi, allié ou employeur.
- **Routes commerciales (décidé)** : des navires marchands PNJ circulent entre les ports des villes côtières, sur des routes visibles sur les cartes.
- **Le joueur peut devenir pirate** et piller les navires marchands, avec des conséquences sur sa réputation auprès des peuples.
- Îles au trésor, cartes au trésor à déchiffrer, épaves à explorer.
- Lien avec les bateaux de la v0.2, les armes steampunk, la discrétion (abordage discret) et les sièges (raids pirates côtiers).
- Point technique : un navire bloc par bloc est une structure mobile, coûteuse à mailler et à simuler ; taille maximale réglable par profil, à mesurer sur le DS.

## v1.8 — Catastrophes naturelles

- Tornades, éruptions volcaniques, inondations, séismes, tempêtes de neige, incendies de forêt.
- **Signes avant-coureurs** à apprendre (animaux qui fuient, ciel qui change, PNJ qui préviennent), dans l'esprit « le savoir est une ressource ».
- Dégâts réels au terrain et aux constructions (lien avec l'intégrité structurelle et le feu).
- **Villages** : les dégâts sont réparables et le village se reconstruit, le joueur peut aider ; mais un village **peut disparaître** si personne n'aide.
- Liées au biome et à la saison.
- **Dimensions (décidé)** : chaque dimension a ses propres catastrophes (éruptions dans l'Infernum, tempêtes dans Machina Caeli, raz-de-marée dans l'Abyssus…).
- **Tout désactivable**, catastrophe par catastrophe, et réglable en fréquence et en intensité.
