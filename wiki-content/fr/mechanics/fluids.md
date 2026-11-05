## Principe

L'eau et la lave **coulent**. Chaque liquide a des **sources** (l'eau des mers et des lacs, les lacs de lave des profondeurs) et des **écoulements** de plus en plus minces : vers le bas d'abord, puis sur les côtés, en filant vers la pente la plus proche. L'eau s'étend jusqu'à 7 blocs sur un sol plat, la lave seulement 3, et bien plus lentement. Retirer la source fait disparaître l'écoulement. L'eau et la lave qui se touchent donnent de la **pierre** ou de l'**obsidienne**.

## À voir

La surface d'un liquide **penche** selon l'écoulement : pleine près de la source, presque à plat au bout. Deux cases du même liquide ne se séparent jamais par une paroi, et un liquide surmonté du même liquide remplit tout son bloc (les cascades). L'eau est translucide, la lave opaque ; leur surface **ondule** doucement, ce que l'option graphique « Animation des liquides » arrête.

## Nager

- On **coule doucement** sans rien toucher.
- **Saut maintenu** : on remonte, jusqu'à flotter à la surface.
- On avance **deux fois moins vite** dans l'eau, encore bien moins dans la lave.
- Contre une berge, un **petit coup de pied** aide à sortir de l'eau.
- La tête dans un liquide, la vue se perd dans un **brouillard** bleu (eau) ou orange (lave).
- En vol créatif, les liquides ne gênent pas.

## Le seau (créatif)

Dans l'onglet **Outils** de l'inventaire créatif :
- le **seau d'eau** et le **seau de lave** posent une source d'un **clic droit**, contre le bloc visé ;
- le **seau** vide, pointé sur une source, la **reprend**.

En créatif, le seau reste tel quel après usage. Le serveur vérifie chaque usage (portée, bloc visé), comme pour poser un bloc.

## Réglages

L'option de monde `fluids` règle l'écoulement (`flow: false` l'arrête) et le nombre de cases de liquide mises à jour à chaque tick (`updates_per_tick`).
