"""
Namify - Application de renommage de fichiers par lot
========================================================
Application de bureau moderne construite avec customtkinter permettant de
renommer un ensemble de fichiers en une seule opération, avec :
  - Ajout de préfixes / suffixes
  - Numérotation automatique séquentielle
  - Recherche & remplacement de texte dans les noms
  - Aperçu en temps réel (ancien nom -> nouveau nom)

Auteur : Namify
"""

import os
import sys
import customtkinter as ctk
from tkinter import filedialog, messagebox


# ----------------------------------------------------------------------------
# Fonctions utilitaires
# ----------------------------------------------------------------------------

def resource_path(relative_path: str) -> str:
    """
    Retourne le chemin absolu vers une ressource (utile pour PyInstaller
    en mode --onefile, où les fichiers sont extraits dans un dossier
    temporaire référencé par sys._MEIPASS).
    """
    try:
        base_path = sys._MEIPASS  # type: ignore[attr-defined]
    except AttributeError:
        base_path = os.path.abspath(".")
    return os.path.join(base_path, relative_path)


# ----------------------------------------------------------------------------
# Classe principale de l'application
# ----------------------------------------------------------------------------

class NamifyApp(ctk.CTk):
    """Fenêtre principale de l'application Namify."""

    def __init__(self):
        super().__init__()

        # ---------------- Configuration générale de la fenêtre ----------------
        self.title("Namify - Renommage de fichiers par lot")
        self.geometry("1050x650")
        self.minsize(900, 550)

        # Icône de la fenêtre (fonctionne sous Windows avec un .ico)
        try:
            icon_path = resource_path("Namify.ico")
            if os.path.exists(icon_path):
                self.iconbitmap(icon_path)
        except Exception:
            # L'icône n'est pas bloquante si elle est absente ou non supportée (Linux/Mac)
            pass

        # ---------------- Variables d'état ----------------
        self.selected_files = []  # Liste des chemins complets des fichiers sélectionnés

        self.prefix_var = ctk.StringVar(value="")
        self.suffix_var = ctk.StringVar(value="")
        self.search_var = ctk.StringVar(value="")
        self.replace_var = ctk.StringVar(value="")

        self.numbering_enabled_var = ctk.BooleanVar(value=False)
        self.start_number_var = ctk.StringVar(value="1")
        self.padding_var = ctk.StringVar(value="2")
        self.separator_var = ctk.StringVar(value="_")
        self.position_var = ctk.StringVar(value="Après le nom")

        # Déclenche l'aperçu en temps réel à chaque modification d'un champ
        for var in (
            self.prefix_var, self.suffix_var, self.search_var, self.replace_var,
            self.numbering_enabled_var, self.start_number_var, self.padding_var,
            self.separator_var, self.position_var,
        ):
            var.trace_add("write", lambda *args: self.update_preview())

        # ---------------- Construction de l'interface ----------------
        self._build_layout()

    # ------------------------------------------------------------------
    # Construction de l'UI
    # ------------------------------------------------------------------

    def _build_layout(self):
        """Construit la structure générale : panneau gauche (options) + panneau droit (aperçu)."""
        self.grid_columnconfigure(0, weight=0)
        self.grid_columnconfigure(1, weight=1)
        self.grid_rowconfigure(0, weight=1)

        self._build_control_panel()
        self._build_preview_panel()

    def _build_control_panel(self):
        """Panneau gauche : sélection des fichiers et options de renommage."""
        panel = ctk.CTkScrollableFrame(self, width=340, corner_radius=12, label_text="Options de renommage")
        panel.grid(row=0, column=0, padx=(15, 8), pady=15, sticky="nsew")

        # ----- Titre / logo texte -----
        title_label = ctk.CTkLabel(panel, text="Namify", font=ctk.CTkFont(size=26, weight="bold"))
        title_label.pack(anchor="w", padx=10, pady=(5, 15))

        # ----- Sélection des fichiers -----
        select_btn = ctk.CTkButton(
            panel, text="📂 Sélectionner des fichiers", command=self.select_files,
        )
        select_btn.pack(fill="x", padx=10, pady=(0, 5))

        self.files_count_label = ctk.CTkLabel(panel, text="Aucun fichier sélectionné", text_color="gray")
        self.files_count_label.pack(anchor="w", padx=10, pady=(0, 15))

        # ----- Section Préfixe / Suffixe -----
        self._section_title(panel, "Préfixe & Suffixe")

        ctk.CTkLabel(panel, text="Préfixe :").pack(anchor="w", padx=10)
        ctk.CTkEntry(panel, textvariable=self.prefix_var, placeholder_text="ex: Vacances_").pack(
            fill="x", padx=10, pady=(0, 8)
        )

        ctk.CTkLabel(panel, text="Suffixe :").pack(anchor="w", padx=10)
        ctk.CTkEntry(panel, textvariable=self.suffix_var, placeholder_text="ex: _final").pack(
            fill="x", padx=10, pady=(0, 15)
        )

        # ----- Section Recherche & Remplacement -----
        self._section_title(panel, "Rechercher & Remplacer")

        ctk.CTkLabel(panel, text="Texte à rechercher :").pack(anchor="w", padx=10)
        ctk.CTkEntry(panel, textvariable=self.search_var, placeholder_text="ex: IMG").pack(
            fill="x", padx=10, pady=(0, 8)
        )

        ctk.CTkLabel(panel, text="Remplacer par :").pack(anchor="w", padx=10)
        ctk.CTkEntry(panel, textvariable=self.replace_var, placeholder_text="ex: Photo").pack(
            fill="x", padx=10, pady=(0, 15)
        )

        # ----- Section Numérotation automatique -----
        self._section_title(panel, "Numérotation automatique")

        numbering_switch = ctk.CTkSwitch(
            panel, text="Activer la numérotation", variable=self.numbering_enabled_var,
            onvalue=True, offvalue=False,
        )
        numbering_switch.pack(anchor="w", padx=10, pady=(0, 10))

        ctk.CTkLabel(panel, text="Position :").pack(anchor="w", padx=10)
        position_menu = ctk.CTkOptionMenu(
            panel, values=["Avant le nom", "Après le nom"], variable=self.position_var,
            command=lambda *_: self.update_preview(),
        )
        position_menu.pack(fill="x", padx=10, pady=(0, 8))

        row_frame = ctk.CTkFrame(panel, fg_color="transparent")
        row_frame.pack(fill="x", padx=10, pady=(0, 8))
        row_frame.grid_columnconfigure((0, 1), weight=1)

        start_frame = ctk.CTkFrame(row_frame, fg_color="transparent")
        start_frame.grid(row=0, column=0, sticky="ew", padx=(0, 5))
        ctk.CTkLabel(start_frame, text="Début à :").pack(anchor="w")
        ctk.CTkEntry(start_frame, textvariable=self.start_number_var, width=90).pack(fill="x")

        padding_frame = ctk.CTkFrame(row_frame, fg_color="transparent")
        padding_frame.grid(row=0, column=1, sticky="ew", padx=(5, 0))
        ctk.CTkLabel(padding_frame, text="Chiffres (ex: 3 -> 001) :").pack(anchor="w")
        ctk.CTkEntry(padding_frame, textvariable=self.padding_var, width=90).pack(fill="x")

        ctk.CTkLabel(panel, text="Séparateur :").pack(anchor="w", padx=10)
        ctk.CTkEntry(panel, textvariable=self.separator_var, width=90).pack(
            anchor="w", padx=10, pady=(0, 20)
        )

        # ----- Boutons d'action -----
        self.apply_btn = ctk.CTkButton(
            panel, text="✅ Appliquer le renommage", fg_color="#2e7d32", hover_color="#1b5e20",
            command=self.apply_renaming,
        )
        self.apply_btn.pack(fill="x", padx=10, pady=(10, 5))

        reset_btn = ctk.CTkButton(
            panel, text="🔄 Réinitialiser", fg_color="transparent", border_width=1,
            command=self.reset_all,
        )
        reset_btn.pack(fill="x", padx=10, pady=(0, 10))

    def _section_title(self, parent, text: str):
        """Ajoute un petit titre de section stylisé dans le panneau."""
        label = ctk.CTkLabel(parent, text=text, font=ctk.CTkFont(size=14, weight="bold"))
        label.pack(anchor="w", padx=10, pady=(5, 5))

    def _build_preview_panel(self):
        """Panneau droit : aperçu en temps réel des renommages."""
        container = ctk.CTkFrame(self, corner_radius=12)
        container.grid(row=0, column=1, padx=(8, 15), pady=15, sticky="nsew")
        container.grid_rowconfigure(1, weight=1)
        container.grid_columnconfigure(0, weight=1)

        header = ctk.CTkLabel(
            container, text="Aperçu en temps réel", font=ctk.CTkFont(size=18, weight="bold")
        )
        header.grid(row=0, column=0, sticky="w", padx=15, pady=(15, 5))

        # En-tête des colonnes
        columns_frame = ctk.CTkFrame(container, fg_color="transparent")
        columns_frame.grid(row=0, column=0, sticky="e", padx=15, pady=(15, 5))

        self.preview_scroll = ctk.CTkScrollableFrame(container, label_text="")
        self.preview_scroll.grid(row=1, column=0, sticky="nsew", padx=15, pady=(0, 15))
        self.preview_scroll.grid_columnconfigure(0, weight=1)
        self.preview_scroll.grid_columnconfigure(1, weight=0)
        self.preview_scroll.grid_columnconfigure(2, weight=1)

        self._draw_empty_preview()

    def _draw_empty_preview(self):
        """Affiche un message d'invite lorsque aucun fichier n'est sélectionné."""
        for widget in self.preview_scroll.winfo_children():
            widget.destroy()
        placeholder = ctk.CTkLabel(
            self.preview_scroll,
            text="Aucun fichier sélectionné.\nCliquez sur « Sélectionner des fichiers » pour commencer.",
            text_color="gray", justify="center",
        )
        placeholder.grid(row=0, column=0, columnspan=3, pady=40)

    # ------------------------------------------------------------------
    # Logique métier
    # ------------------------------------------------------------------

    def select_files(self):
        """Ouvre une boîte de dialogue pour sélectionner plusieurs fichiers."""
        files = filedialog.askopenfilenames(title="Sélectionner des fichiers à renommer")
        if files:
            self.selected_files = list(files)
            count = len(self.selected_files)
            self.files_count_label.configure(
                text=f"{count} fichier(s) sélectionné(s)", text_color=("black", "white")
            )
            self.update_preview()

    def build_new_name(self, filename: str, index: int) -> str:
        """
        Construit le nouveau nom de fichier en appliquant, dans l'ordre :
        1. Recherche & remplacement
        2. Numérotation automatique (avant ou après le nom)
        3. Préfixe et suffixe
        L'extension d'origine est toujours préservée.
        """
        name, ext = os.path.splitext(filename)

        # 1. Recherche & remplacement
        search_text = self.search_var.get()
        replace_text = self.replace_var.get()
        if search_text:
            name = name.replace(search_text, replace_text)

        # 2. Numérotation automatique
        if self.numbering_enabled_var.get():
            try:
                start = int(self.start_number_var.get())
            except ValueError:
                start = 1
            try:
                padding = int(self.padding_var.get())
                padding = max(padding, 1)
            except ValueError:
                padding = 2

            number_str = str(start + index).zfill(padding)
            separator = self.separator_var.get()

            if self.position_var.get() == "Avant le nom":
                name = f"{number_str}{separator}{name}"
            else:
                name = f"{name}{separator}{number_str}"

        # 3. Préfixe et suffixe
        prefix = self.prefix_var.get()
        suffix = self.suffix_var.get()
        new_name = f"{prefix}{name}{suffix}{ext}"

        return new_name

    def update_preview(self):
        """Recalcule et réaffiche l'aperçu ancien nom -> nouveau nom pour chaque fichier."""
        for widget in self.preview_scroll.winfo_children():
            widget.destroy()

        if not self.selected_files:
            self._draw_empty_preview()
            return

        # En-têtes de colonnes
        header_old = ctk.CTkLabel(self.preview_scroll, text="Nom actuel", font=ctk.CTkFont(weight="bold"))
        header_old.grid(row=0, column=0, sticky="w", padx=(5, 10), pady=(0, 8))
        header_arrow = ctk.CTkLabel(self.preview_scroll, text="")
        header_arrow.grid(row=0, column=1)
        header_new = ctk.CTkLabel(self.preview_scroll, text="Nouveau nom", font=ctk.CTkFont(weight="bold"))
        header_new.grid(row=0, column=2, sticky="w", padx=(10, 5), pady=(0, 8))

        for index, full_path in enumerate(self.selected_files):
            old_name = os.path.basename(full_path)
            new_name = self.build_new_name(old_name, index)

            row = index + 1

            old_label = ctk.CTkLabel(self.preview_scroll, text=old_name, anchor="w", justify="left")
            old_label.grid(row=row, column=0, sticky="ew", padx=(5, 10), pady=3)

            arrow_label = ctk.CTkLabel(self.preview_scroll, text="➜")
            arrow_label.grid(row=row, column=1, padx=5, pady=3)

            # Met en évidence en vert si le nom change, sinon couleur neutre
            new_color = "#2e7d32" if new_name != old_name else ("black", "white")
            new_label = ctk.CTkLabel(
                self.preview_scroll, text=new_name, anchor="w", justify="left",
                text_color=new_color, font=ctk.CTkFont(weight="bold"),
            )
            new_label.grid(row=row, column=2, sticky="ew", padx=(10, 5), pady=3)

    def apply_renaming(self):
        """Applique effectivement le renommage sur le disque, avec gestion des erreurs et conflits."""
        if not self.selected_files:
            messagebox.showwarning("Namify", "Veuillez d'abord sélectionner des fichiers.")
            return

        # Construction de la liste des renommages (chemin_source -> chemin_cible)
        renaming_plan = []
        for index, full_path in enumerate(self.selected_files):
            directory = os.path.dirname(full_path)
            old_name = os.path.basename(full_path)
            new_name = self.build_new_name(old_name, index)
            new_path = os.path.join(directory, new_name)
            renaming_plan.append((full_path, new_path))

        # Confirmation avant application
        confirm = messagebox.askyesno(
            "Namify - Confirmation",
            f"Vous êtes sur le point de renommer {len(renaming_plan)} fichier(s).\n"
            "Cette action est irréversible. Continuer ?",
        )
        if not confirm:
            return

        success_count = 0
        errors = []

        # Étape 1 : renommage temporaire pour éviter les collisions de noms entre fichiers du lot
        temp_plan = []
        for i, (src, dst) in enumerate(renaming_plan):
            temp_path = os.path.join(os.path.dirname(src), f"__namify_tmp_{i}__{os.path.basename(src)}")
            try:
                os.rename(src, temp_path)
                temp_plan.append((temp_path, dst))
            except Exception as e:
                errors.append(f"{os.path.basename(src)} : {e}")

        # Étape 2 : renommage final vers le nom cible
        for temp_path, dst in temp_plan:
            try:
                if os.path.exists(dst):
                    raise FileExistsError("un fichier portant ce nom existe déjà")
                os.rename(temp_path, dst)
                success_count += 1
            except Exception as e:
                errors.append(f"{os.path.basename(temp_path)} : {e}")

        # Message de confirmation final
        if errors:
            error_text = "\n".join(errors[:10])
            messagebox.showwarning(
                "Namify - Terminé avec des erreurs",
                f"{success_count} fichier(s) renommé(s) avec succès.\n"
                f"{len(errors)} erreur(s) rencontrée(s) :\n{error_text}",
            )
        else:
            messagebox.showinfo(
                "Namify - Succès",
                f"{success_count} fichier(s) renommé(s) avec succès !",
            )

        # Réinitialisation de la sélection après traitement
        self.selected_files = []
        self.files_count_label.configure(text="Aucun fichier sélectionné", text_color="gray")
        self.update_preview()

    def reset_all(self):
        """Réinitialise tous les champs et la sélection de fichiers."""
        self.selected_files = []
        self.prefix_var.set("")
        self.suffix_var.set("")
        self.search_var.set("")
        self.replace_var.set("")
        self.numbering_enabled_var.set(False)
        self.start_number_var.set("1")
        self.padding_var.set("2")
        self.separator_var.set("_")
        self.position_var.set("Après le nom")
        self.files_count_label.configure(text="Aucun fichier sélectionné", text_color="gray")
        self.update_preview()


# ----------------------------------------------------------------------------
# Point d'entrée
# ----------------------------------------------------------------------------

def main():
    # Thème adaptatif : suit automatiquement le mode clair/sombre du système
    ctk.set_appearance_mode("System")
    ctk.set_default_color_theme("blue")

    app = NamifyApp()
    app.mainloop()


if __name__ == "__main__":
    main()
