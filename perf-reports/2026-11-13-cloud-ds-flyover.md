# Rapport de performance — `flyover` — profil `ds`

> **Cloud, lavapipe, indicatif** : rendu logiciel sur une VM partagée. Les FPS ne valent rien ; seules la mémoire et la tendance comptent. La VM était ce jour-là ~15-20 % plus lente que le 11/11 (le code de la v0.2.0 remesuré dessus donne des bancs CPU 15-20 % plus longs qu'à sa propre clôture). Graine aléatoire (monde différent du rapport du 11/11). Lancement → carte graphique prête : 0,5 s. Chargement initial du monde ≈ 10,3 s d'après le journal (démarrage du serveur → rapport : 133,2 s − 3 s de préchauffage − 119,9 s de mesure). Pic de mémoire 301 Mio (241 Mio le 11/11) : la v0.2.1 fabrique toutes les variantes de sons au chargement (~20 Mo) et le monde n'est pas le même.

| Contexte | Valeur |
|---|---|
| Date | 2026-11-13 |
| Version | 0.2.1 (fin de version) |
| Machine | vm |
| Système | Linux (Ubuntu 24.04) |
| Processeur | Intel(R) Xeon(R) Processor @ 2.10GHz (4 threads) |
| Mémoire totale | 16086 Mio |
| Carte graphique | llvmpipe (LLVM 20.1.2, 256 bits) (llvmpipe, Cpu) |
| Résolution | 1280×720 |
| Présentation | Mailbox |
| Durée mesurée | 119.9 s, 4486 images |

| Mesure | Valeur |
|---|---|
| FPS moyen | 37.4 |
| 1 % low (FPS) | 12.6 |
| 0,1 % low (FPS) | 10.4 |
| Temps de frame moyen | 26.75 ms |
| Temps CPU moyen / 1 % pire | 0.75 ms / 2.11 ms |
| Temps GPU moyen / 1 % pire | n/d / n/d |
| Mémoire (pic, RSS) | 301 Mio |
| Ticks serveur par seconde | 20.0 |
