# Installation sur le DS (Linux maison)

Ce qu'il faut sur un Linux fait maison (Intel N100) pour lancer Strata Mundi, puis comment le clapet du DS est géré.

## Dépendances système

### À quoi ça sert

Le binaire `strata` est autonome (aucune bibliothèque Rust à installer), mais il s'appuie sur quelques bibliothèques du système : le pilote graphique, la fenêtre, les manettes.

### Ce qu'il faut installer

| Rôle | Bibliothèque (nom Debian/Ubuntu) | Chargée comment |
|---|---|---|
| Vulkan (chargeur) | `libvulkan1` | à l'exécution (`dlopen`) |
| Pilote Vulkan Intel (N100 : « ANV ») | `mesa-vulkan-drivers` (Mesa ≥ 23) | par le chargeur Vulkan |
| Fenêtre Wayland | `libwayland-client0`, `libxkbcommon0` | à l'exécution |
| Fenêtre X11 (si pas de Wayland) | `libx11-6`, `libxcursor1`, `libxrandr2`, `libxi6`, `libxkbcommon-x11-0` | à l'exécution |
| Manettes | `libudev1` | liée au binaire |
| Clapet (veille) | D-Bus système + `systemd-logind` ou `elogind` | facultatif |
| Son | rien encore : l'audio arrive avec la v0.2 (ALSA/PipeWire via `libasound2`) | — |

Vérifications rapides :

- `vulkaninfo --summary` (paquet `vulkan-tools`) doit lister **Intel(R) Graphics (ADL-N)** avec le pilote `Intel open-source Mesa driver`. Si seul `llvmpipe` apparaît, le pilote Intel manque : le jeu tournera, mais en rendu logiciel (très lent).
- Manette : l'utilisateur doit pouvoir lire `/dev/input/event*`. Sur un système minimal, ajouter l'utilisateur au groupe `input` (`usermod -aG input <nom>`) ou installer la règle udev de la distribution pour les manettes. `evtest` permet de vérifier que les boutons arrivent.
- Écran : n'importe quel compositeur Wayland (ou un serveur X) ; le jeu ouvre une fenêtre plein écran via `winit`. Sans compositeur, `cage` (kiosque Wayland) suffit : `cage -- strata`.

### Réglages conseillés pour le DS

- Choisir le préréglage **DS** dans Options → Graphismes → Préréglage (30 images/s, échelle de rendu 75 %, distance 6, un seul fil de maillage) : il est gardé dans `config/settings.toml`. `strata --preset ds` l'applique pour un lancement sans l'écrire.
- La console du DS n'a pas de clavier : tout le jeu se joue à la manette, sauf la console de commandes.
- Les mesures d'images par seconde se font sur la machine elle-même : `strata --bench-scene flyover --preset ds` écrit un rapport dans `perf-reports/`.

### Où sont les fichiers

`~/.local/share/strata-mundi/` (ou `--data-dir <dossier>`) : `saves/` (mondes), `backups/` (copies), `config/settings.toml` (réglages), `crash-reports/` (rapports de plantage).

## Fermeture du clapet

### À quoi ça sert

Refermer le DS doit mettre la partie en pause et la sauvegarder tout de suite ; le rouvrir reprend la partie là où elle était, sans menu à traverser.

### Comment ça marche

Sous Linux, c'est **logind** (le gestionnaire de sessions de systemd, ou `elogind` sur un système sans systemd) qui met la machine en veille. Juste avant, il envoie le signal D-Bus `PrepareForSleep(true)` ; au réveil, `PrepareForSleep(false)`.

1. Au démarrage, le jeu prend un **verrou de retardement** (*delay inhibitor lock*) : logind attendra que le jeu le relâche avant de mettre en veille, au plus `InhibitDelayMaxSec` secondes (5 par défaut).
2. Clapet fermé → `PrepareForSleep(true)` → le jeu met le monde en pause, demande une sauvegarde (avec une capture pour la liste des mondes), attend la confirmation du serveur (4 s au plus), puis relâche le verrou : la machine s'endort.
3. Clapet ouvert → `PrepareForSleep(false)` → la partie reprend et le jeu reprend un verrou pour la prochaine fois.

Sans logind (système sans D-Bus, conteneur), le jeu le note dans son journal et continue normalement : seule la sauvegarde automatique (toutes les 5 minutes) protège alors la partie.

### Ce qu'il faut sur le système

- **D-Bus système** et **logind** (`systemd-logind` ou `elogind`) qui gère la mise en veille (pour le clapet : `HandleLidSwitch=suspend` dans `/etc/systemd/logind.conf` ou `/etc/elogind/logind.conf`).
- Si 5 secondes ne suffisent pas pour sauvegarder (gros monde sur une carte SD lente), augmenter `InhibitDelayMaxSec` dans le même fichier.
- Pour vérifier : `busctl monitor org.freedesktop.login1` puis fermer le clapet ; le signal `PrepareForSleep` doit apparaître.

## Où c'est dans le code

- `crates/client/src/sleep.rs` (`SleepWatcher`) : écoute logind sur un fil d'exécution séparé, avec la crate `zbus`.
- `crates/client/src/session/transitions.rs` (`watch_sleep`) : pause, sauvegarde, puis feu vert.

<details><summary>Côté Rust</summary>

- **`#[cfg(target_os = "linux")]`** : ce code n'est compilé que sous Linux ; sous Windows, une version vide le remplace et le jeu fonctionne sans.
- **Canaux `mpsc`** : le fil d'écoute envoie « on s'endort » au fil principal, qui répond « c'est sauvegardé » par un second canal.
- **`OwnedFd`** : le verrou de logind est un descripteur de fichier ; le **relâcher** consiste simplement à le fermer (`drop`).

</details>
