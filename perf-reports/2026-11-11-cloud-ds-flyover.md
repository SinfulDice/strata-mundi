# Rapport de performance — `flyover` — profil `ds`

> **Cloud, lavapipe, indicatif** : rendu logiciel sur une VM partagée. Les FPS ne valent rien ; seules la mémoire et la tendance comptent. Lancement → carte graphique prête : 1,1 s. Chargement initial du monde ≈ 9 s d'après le journal (démarrage du serveur → début de la scène : 132,0 s − 3 s de préchauffage − 120 s de mesure) ; 10,9 s avec la méthode précédente (durée totale du processus, Xvfb et fermeture compris).

| Contexte | Valeur |
|---|---|
| Date | 2026-11-11 |
| Version | v0.2 (fin de version, binaire construit avant le passage du numéro à 0.2.0) |
| Machine | vm |
| Système | Linux (Ubuntu 24.04) |
| Processeur | Intel(R) Xeon(R) Processor @ 2.10GHz (4 threads) |
| Mémoire totale | 16086 Mio |
| Carte graphique | llvmpipe (LLVM 20.1.2, 256 bits) (llvmpipe, Cpu) |
| Résolution | 1280×720 |
| Présentation | Mailbox |
| Durée mesurée | 120.0 s, 7021 images |

| Mesure | Valeur |
|---|---|
| FPS moyen | 58.5 |
| 1 % low (FPS) | 16.9 |
| 0,1 % low (FPS) | 13.6 |
| Temps de frame moyen | 17.09 ms |
| Temps CPU moyen / 1 % pire | 0.73 ms / 1.76 ms |
| Temps GPU moyen / 1 % pire | n/d / n/d |
| Mémoire (pic, RSS) | 241 Mio |
| Ticks serveur par seconde | 20.0 |

> v0.1 : les profils règlent pour l'instant les distances de rendu, l'occlusion ambiante et les threads ; les autres réglages arrivent au jalon 8.
