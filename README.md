# Namify

Dépôt GitHub : **https://github.com/Les-Doubles-A/Namify**

Application de bureau pour renommer des fichiers et des dossiers **par lot**, avec aperçu en temps réel, détection des conflits et annulation. Depuis la version 2.0, Namify est entièrement réécrit en **Rust** (interface `egui`) : un seul exécutable natif, rapide, sans dépendance à installer.

## Installation

Téléchargez **`Namify-Setup-2.0.0.exe`** depuis la [dernière release](https://github.com/Les-Doubles-A/Namify/releases/latest), puis lancez-le.

- Installation pour l'utilisateur courant, sans droits administrateur (par défaut dans `%LOCALAPPDATA%\Programs\Namify`).
- Raccourci dans le menu Démarrer, icône de bureau en option.
- Intégration à l'explorateur (options cochées par défaut) :
  - **Envoyer vers > Namify** sur une sélection de fichiers ;
  - **Renommer avec Namify** au clic droit sur un dossier ou dans le fond d'un dossier ouvert.
- Met à jour une installation 1.x existante. Désinstallation depuis les Paramètres Windows.

Une version portable (`Namify.exe` seul) est aussi jointe à la release.

L'installeur n'est pas signé : si Windows SmartScreen l'affiche, cliquez sur « Informations complémentaires » puis « Exécuter quand même ».

## Fonctionnalités

### Pipeline de règles

Les règles s'empilent et s'appliquent dans l'ordre. Chacune peut être activée, désactivée, repliée, dupliquée, déplacée ou supprimée.

| Règle | Ce qu'elle fait |
|---|---|
| Rechercher et remplacer | Texte simple ou expression régulière (groupes `$1`, `${nom}`), sensible ou non à la casse, toutes / première / dernière occurrence, sur le nom, l'extension ou les deux |
| Insérer du texte | Au début, à la fin, à une position (négative = depuis la fin), avant ou après un texte repère ; balises acceptées |
| Supprimer | N premiers / derniers caractères, une plage, un texte, tous les chiffres, toutes les lettres, le contenu entre deux délimiteurs, tout avant / après un texte |
| Casse | minuscules, MAJUSCULES, Titre, Phrase, inversée, camelCase, PascalCase, snake_case, kebab-case, SCREAMING_SNAKE |
| Numérotation | Décimale, lettres (a, b... aa), chiffres romains, hexadécimal ; début, pas, nombre de chiffres, position, séparateur, remise à zéro par dossier |
| Modèle (balises) | Reconstruit tout le nom à partir de balises : `{name}_{n:3}`, `{exif:%Y-%m-%d}`... |
| Nettoyage | Rogner, espaces multiples, `_` et `.` en espaces, accents, caractères spéciaux, ASCII seul, contenu entre crochets, caractères interdits par Windows, remplacement des espaces |
| Extension | Minuscules, majuscules, remplacer, supprimer, ajouter |
| Tronquer | Longueur maximale, en gardant le début ou la fin |
| Permuter | « Artiste - Titre » devient « Titre - Artiste » |
| Compléter les nombres | `img7` devient `img007` pour un tri naturel |

### Balises dynamiques

Utilisables dans « Insérer », « Remplacer par » et « Modèle » (bouton `{}` à côté du champ) :

`{name}`, `{name:0:5}`, `{cur}`, `{ext}`, `{full}`, `{n}`, `{n:3}`, `{fn:2}` (compteur par dossier), `{total}`, `{parent}`, `{parent:2}`, `{date:%Y-%m-%d}`, `{mdate}`, `{cdate}`, `{exif}` (date de prise de vue, sinon date de modification), `{make}`, `{model}`, `{width}`, `{height}`, `{dims}`, `{size}`, `{bytes}`, `{crc}`. Les formats de date suivent la syntaxe strftime.

### Liste de fichiers

- Ajout par boutons, glisser-déposer, arguments de ligne de commande, « Envoyer vers » ou clic droit sur un dossier.
- Dossiers : contenu seul ou récursif, fichiers cachés inclus ou non, filtre par extensions, ou renommage des dossiers eux-mêmes.
- Aperçu ancien nom / nouveau nom avec mise en évidence de la partie modifiée.
- Statut par ligne : prêt, inchangé, doublon dans le lot, existe déjà sur le disque, nom invalide pour Windows (caractères interdits, noms réservés, longueur, point final...).
- Résolution automatique des conflits en option (« (2) », « (3) »...).
- Filtre texte, filtre par statut, tri (naturel, alphabétique, extension, taille, dates, dossier) ou ordre manuel.
- Cases à cocher par fichier, sélection multiple (Ctrl / Maj), menu contextuel (ouvrir, afficher dans l'explorateur, copier le chemin ou le nouveau nom).
- Panneau de détails : métadonnées, EXIF, dimensions, CRC32 et valeur de chaque balise pour le fichier sélectionné.
- Export de l'aperçu en CSV.

### Sécurité

- Renommage en deux passes via des noms temporaires : les permutations (a <-> b) et les changements de casse seuls fonctionnent.
- Fichiers renommés avant leurs dossiers parents, retour arrière automatique d'un élément en cas d'échec.
- Historique persistant des 100 derniers lots : chaque lot peut être annulé, même après redémarrage (Ctrl+Z pour le dernier).

### Confort

- Préréglages : enregistrer, charger, ajouter à la suite, supprimer, importer / exporter en JSON. Cinq préréglages fournis (photos par date EXIF, noms propres pour le web, épisodes de série, numérotation en préfixe, suppression de « (1) » / « - Copie »).
- Thème clair, sombre ou système, 8 couleurs d'accent ou couleur libre, zoom de l'interface.
- Restauration des règles au démarrage, réglages persistants dans `%APPDATA%\Les-Doubles-A\Namify\config`.
- Raccourcis : Ctrl+O, Ctrl+Maj+O, Ctrl+Entrée, Ctrl+Z, Ctrl+F, Ctrl+A, Ctrl+S, Ctrl+E, Espace, Suppr, Alt+Haut / Bas, F5, F1.

## Développement

Prérequis : [Rust](https://rustup.rs) stable (édition 2024) et, sous Windows, les outils MSVC.

```bash
cargo run                 # lancer en mode debug
cargo run -- C:\Photos    # lancer avec des fichiers ou dossiers pré-chargés
cargo test                # tests unitaires (règles, balises, moteur de renommage)
cargo build --release     # exécutable optimisé : target\release\Namify.exe
```

### Structure

```
Namify/
├── Cargo.toml
├── build.rs              # icône et métadonnées de l'exécutable Windows
├── Namify.ico / .png     # icônes
├── assets/               # police d'icônes Phosphor (MIT)
├── installer/Namify.iss  # script Inno Setup
└── src/
    ├── main.rs           # point d'entrée, fenêtre
    ├── app.rs            # état, actions et mise en page
    ├── rules.rs          # règles de renommage (logique pure, testée)
    ├── rules_ui.rs       # éditeurs des règles
    ├── tokens.rs         # balises dynamiques
    ├── engine.rs         # aperçu, conflits, exécution, annulation
    ├── meta.rs           # métadonnées, EXIF, dimensions, CRC32
    ├── storage.rs        # réglages, préréglages, historique, session
    ├── theme.rs          # palette, polices
    ├── widgets.rs        # composants d'interface
    └── icons.rs          # codes des icônes Phosphor
```

### Recréer l'installeur

```bash
cargo build --release
iscc installer\Namify.iss
```

Le setup est créé dans `installer\Output\` ([Inno Setup 6](https://jrsoftware.org/isinfo.php) requis).

## Licences tierces

Icônes : [Phosphor Icons](https://phosphoricons.com), licence MIT, fichier de police repris du crate `egui-phosphor` (voir `assets/Phosphor-LICENSE.txt`).
