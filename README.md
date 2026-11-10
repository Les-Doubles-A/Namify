# Namify

Dépôt GitHub : **https://github.com/Les-Doubles-A/Namify**

Application de bureau pour renommer des fichiers **par lot**, avec aperçu en temps réel.
Interface construite avec `customtkinter`, thème clair/sombre automatique (suit l'OS).

## Fonctionnalités

- Sélection multiple de fichiers via boîte de dialogue
- Ajout de préfixe / suffixe
- Numérotation automatique séquentielle (position, point de départ, nombre de chiffres, séparateur)
- Recherche & remplacement de texte dans les noms
- Aperçu en temps réel (ancien nom ➜ nouveau nom) avant validation
- Renommage sécurisé (passage par des noms temporaires pour éviter les conflits) + message de confirmation

## Structure du projet

```
Namify/
├── main.py            # Script principal de l'application
├── convert_icon.py     # Utilitaire de conversion Namify.png -> Namify.ico
├── requirements.txt    # Dépendances Python
├── Namify.png          # Icône source (à fournir)
├── Namify.ico          # Icône générée (produite par convert_icon.py)
└── README.md
```

## 1. Installation des dépendances

Créez un environnement virtuel (recommandé) puis installez les paquets :

```bash
python -m venv venv
# Windows
venv\Scripts\activate
# macOS / Linux
source venv/bin/activate

pip install customtkinter pillow pyinstaller
```

(ou directement : `pip install -r requirements.txt`)

## 2. Lancer l'application en mode développement

```bash
python main.py
```

## 3. Générer l'icône .ico à partir du .png

Placez votre fichier `Namify.png` (idéalement carré, ≥ 256x256 px) à la racine du dossier `Namify/`,
puis lancez :

```bash
python convert_icon.py
```

Cela produit `Namify.ico`, utilisé à la fois par la fenêtre de l'application (icône de titre)
et par l'exécutable final.

## 4. Compiler en exécutable unique (.exe)

Depuis le dossier `Namify/` (celui qui contient `main.py` et `Namify.ico`), lancez :

```bash
pyinstaller --noconsole --onefile --icon=Namify.ico --name=Namify main.py
```

Options :
- `--noconsole` : aucune fenêtre console ne s'ouvre (application graphique pure)
- `--onefile` : produit un seul fichier exécutable
- `--icon=Namify.ico` : intègre l'icône à l'exécutable
- `--name=Namify` : nom du fichier final (`Namify.exe`)

L'exécutable final se trouve ensuite dans le dossier `dist/Namify.exe`.

> ⚠️ Si l'application utilise `Namify.ico` au runtime (icône de la fenêtre), pensez à
> l'ajouter aux données embarquées avec `--add-data "Namify.ico;."` (Windows) ou
> `--add-data "Namify.ico:."` (macOS/Linux) si vous constatez qu'elle ne s'affiche pas
> une fois compilé.

Commande complète recommandée si l'icône doit être accessible au runtime :

```bash
pyinstaller --noconsole --onefile --icon=Namify.ico --add-data "Namify.ico;." --name=Namify main.py
```

## Notes techniques

- Le renommage se fait en deux passes (noms temporaires puis noms finaux) afin d'éviter
  les collisions lorsque plusieurs fichiers échangent des portions de nom (ex: numérotation).
- Tous les champs sont reliés à l'aperçu via des `trace` sur les variables `customtkinter`,
  garantissant une mise à jour instantanée à chaque frappe.
- Le thème suit automatiquement le mode clair/sombre de l'OS grâce à
  `ctk.set_appearance_mode("System")`.

<!-- installeur-release -->
## Installation

Téléchargez **`Namify-Setup-1.0.0.exe`** depuis la [dernière release](https://github.com/Les-Doubles-A/Namify/releases/latest), puis lancez-le.

L'installation se fait pour l'utilisateur courant, sans droits administrateur (par défaut dans `%LOCALAPPDATA%\Programs\Namify`), avec un raccourci dans le menu Démarrer et une désinstallation depuis les Paramètres Windows.

L'installeur n'est pas signé : si Windows SmartScreen l'affiche, cliquez sur « Informations complémentaires » puis « Exécuter quand même ».

### Recréer l'installeur

Compiler l'application (`pyinstaller Namify.spec`), puis lancer [Inno Setup 6](https://jrsoftware.org/isinfo.php) : `iscc installer\Namify.iss`. Le setup est créé dans `installer\Output\`.
