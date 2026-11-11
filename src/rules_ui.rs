//! Éditeurs des règles dans le panneau « Pipeline ».

use eframe::egui::{self, RichText, Ui};

use crate::icons as ic;
use crate::rules::*;
use crate::theme::{self, Palette};
use crate::tokens;
use crate::widgets::*;

pub enum CardAction {
    None,
    Up,
    Down,
    Duplicate,
    Delete,
}

/// Bouton « {} » qui insère une balise à la fin du champ.
fn token_menu(ui: &mut Ui, target: &mut String) {
    ui.menu_button(RichText::new(ic::BRACKETS_CURLY).size(14.0), |ui| {
        ui.set_min_width(330.0);
        ui.label(RichText::new("Insérer une balise").strong());
        ui.separator();
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (tok, desc) in tokens::HELP {
                let insert = tok.split(" /").next().unwrap_or(tok).split(" et").next().unwrap_or(tok);
                if ui
                    .add(egui::Button::new(format!("{insert}")).right_text(RichText::new(*desc).small().weak()).frame_when_inactive(false))
                    .clicked()
                {
                    let t = if insert == "{{" { "{{" } else { insert };
                    target.push_str(t);
                    ui.close();
                }
            }
        });
    })
    .response
    .on_hover_text("Insérer une balise dynamique");
}

fn grid(ui: &mut Ui, id: egui::Id, body: impl FnOnce(&mut Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 7.0]).min_col_width(92.0).show(ui, body);
}

fn row_label(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).color(p.weak));
}

fn field_with_tokens(ui: &mut Ui, value: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        let w = ui.available_width() - 34.0;
        ui.add(egui::TextEdit::singleline(value).hint_text(hint).desired_width(w.max(60.0)));
        token_menu(ui, value);
    });
}

/// Dessine une carte de règle. Retourne l'action demandée par l'utilisateur.
pub fn rule_card(ui: &mut Ui, p: &Palette, item: &mut RuleItem, idx: usize, count: usize, error: Option<&str>) -> CardAction {
    let mut action = CardAction::None;
    let id = ui.scope_id().with(("rule", idx));
    let kind = item.rule.kind();
    let frame = card_frame(p).stroke(egui::Stroke::new(
        1.0,
        if error.is_some() {
            p.error
        } else if item.enabled {
            p.border
        } else {
            p.border.gamma_multiply(0.6)
        },
    ));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let caret = if item.collapsed { ic::CARET_RIGHT } else { ic::CARET_DOWN };
            if small_icon_button(ui, caret, if item.collapsed { "Déplier" } else { "Replier" }).clicked() {
                item.collapsed = !item.collapsed;
            }
            toggle(ui, p, &mut item.enabled).on_hover_text("Activer / désactiver la règle");
            let color = if item.enabled { p.accent } else { p.weak };
            ui.label(RichText::new(kind.icon()).size(16.0).color(color));
            // Boutons à droite d'abord, puis le titre tronqué dans l'espace restant.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().button_padding = egui::vec2(4.0, 2.0);
                ui.spacing_mut().item_spacing.x = 2.0;
                if small_icon_button(ui, ic::TRASH, "Supprimer la règle").clicked() {
                    action = CardAction::Delete;
                }
                if small_icon_button(ui, ic::COPY, "Dupliquer").clicked() {
                    action = CardAction::Duplicate;
                }
                if small_icon_button_enabled(ui, idx + 1 < count, ic::ARROW_DOWN, "Descendre").clicked() {
                    action = CardAction::Down;
                }
                if small_icon_button_enabled(ui, idx > 0, ic::ARROW_UP, "Monter").clicked() {
                    action = CardAction::Up;
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let title = ui.add(
                        egui::Label::new(
                            RichText::new(format!("{}. {}", idx + 1, kind.title()))
                                .font(theme::bold(14.0))
                                .color(if item.enabled { p.text } else { p.weak }),
                        )
                        .truncate()
                        .sense(egui::Sense::click()),
                    );
                    if title.clicked() {
                        item.collapsed = !item.collapsed;
                    }
                });
            });
        });
        if item.collapsed {
            ui.add(egui::Label::new(RichText::new(item.rule.summary()).small().color(p.weak)).truncate());
        } else {
            ui.add_space(4.0);
            ui.add_enabled_ui(item.enabled, |ui| editor(ui, p, &mut item.rule, id));
        }
        if let Some(e) = error {
            ui.add_space(2.0);
            ui.label(RichText::new(format!("{}  {e}", ic::WARNING)).color(p.error).small());
        }
    });
    action
}

fn editor(ui: &mut Ui, p: &Palette, rule: &mut Rule, id: egui::Id) {
    match rule {
        Rule::Replace { find, replace, regex, case_sensitive, scope, occurrence } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Rechercher");
                text_field(ui, find, if *regex { r"ex : (\d+)-(\w+)" } else { "ex : IMG_" });
                ui.end_row();
                row_label(ui, p, "Remplacer par");
                ui.vertical(|ui| field_with_tokens(ui, replace, if *regex { "ex : $2_$1" } else { "vide = supprimer" }));
                ui.end_row();
                row_label(ui, p, "Occurrence");
                enum_combo(ui, id.with("occ"), occurrence, &Occurrence::ALL, Occurrence::label, 140.0);
                ui.end_row();
                row_label(ui, p, "Appliquer à");
                enum_combo(ui, id.with("scope"), scope, &Scope::ALL, Scope::label, 140.0);
                ui.end_row();
            });
            ui.horizontal(|ui| {
                ui.checkbox(regex, "Expression régulière");
                ui.checkbox(case_sensitive, "Respecter la casse");
            });
        }
        Rule::Insert { text, position, index, anchor, scope_ext } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Texte");
                ui.vertical(|ui| field_with_tokens(ui, text, "ex : {mdate}_ ou Vacances_"));
                ui.end_row();
                row_label(ui, p, "Position");
                enum_combo(ui, id.with("pos"), position, &Position::ALL, Position::label, 160.0);
                ui.end_row();
                match position {
                    Position::Index => {
                        row_label(ui, p, "Index");
                        ui.add(egui::DragValue::new(index).range(-255..=255)).on_hover_text("Négatif = depuis la fin");
                        ui.end_row();
                    }
                    Position::BeforeText | Position::AfterText => {
                        row_label(ui, p, "Texte repère");
                        text_field(ui, anchor, "ex : -");
                        ui.end_row();
                    }
                    _ => {}
                }
            });
            ui.checkbox(scope_ext, "Insérer dans l'extension plutôt que le nom");
        }
        Rule::Remove { mode, count, from, text, text2, case_sensitive } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Supprimer");
                enum_combo(ui, id.with("mode"), mode, &RemoveMode::ALL, RemoveMode::label, 190.0);
                ui.end_row();
                match mode {
                    RemoveMode::FirstN | RemoveMode::LastN => {
                        row_label(ui, p, "Nombre");
                        ui.add(egui::DragValue::new(count).range(0..=255));
                        ui.end_row();
                    }
                    RemoveMode::Range => {
                        row_label(ui, p, "À partir de");
                        ui.add(egui::DragValue::new(from).range(0..=255)).on_hover_text("Position du premier caractère (0 = début)");
                        ui.end_row();
                        row_label(ui, p, "Nombre");
                        ui.add(egui::DragValue::new(count).range(0..=255));
                        ui.end_row();
                    }
                    RemoveMode::Text | RemoveMode::BeforeText | RemoveMode::AfterText => {
                        row_label(ui, p, "Texte");
                        text_field(ui, text, "ex : - Copie");
                        ui.end_row();
                    }
                    RemoveMode::Between => {
                        row_label(ui, p, "Début");
                        text_field(ui, text, "ex : (");
                        ui.end_row();
                        row_label(ui, p, "Fin");
                        text_field(ui, text2, "ex : )");
                        ui.end_row();
                    }
                    _ => {}
                }
            });
            if matches!(mode, RemoveMode::Text | RemoveMode::Between | RemoveMode::BeforeText | RemoveMode::AfterText) {
                ui.checkbox(case_sensitive, "Respecter la casse");
            }
        }
        Rule::Case { mode, scope } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Style");
                enum_combo(ui, id.with("case"), mode, &CaseMode::ALL, CaseMode::label, 190.0);
                ui.end_row();
                row_label(ui, p, "Appliquer à");
                enum_combo(ui, id.with("scope"), scope, &Scope::ALL, Scope::label, 190.0);
                ui.end_row();
            });
        }
        Rule::Numbering { start, step, padding, position, index, separator, style, reset_per_folder } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Style");
                enum_combo(ui, id.with("style"), style, &NumStyle::ALL, NumStyle::label, 150.0);
                ui.end_row();
                row_label(ui, p, "Début / pas");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(start).range(-1_000_000..=1_000_000));
                    ui.add(egui::DragValue::new(step).range(-1000..=1000).prefix("pas "));
                });
                ui.end_row();
                row_label(ui, p, "Chiffres");
                ui.add(egui::DragValue::new(padding).range(0..=12)).on_hover_text("3 donne 001, 002...");
                ui.end_row();
                row_label(ui, p, "Position");
                enum_combo(ui, id.with("pos"), position, &Position::SIMPLE, Position::label, 150.0);
                ui.end_row();
                if *position == Position::Index {
                    row_label(ui, p, "Index");
                    ui.add(egui::DragValue::new(index).range(-255..=255));
                    ui.end_row();
                } else {
                    row_label(ui, p, "Séparateur");
                    text_field(ui, separator, "ex : _ ou  - ");
                    ui.end_row();
                }
            });
            ui.checkbox(reset_per_folder, "Recommencer à chaque dossier");
        }
        Rule::Clean {
            trim,
            collapse_spaces,
            separators_to_spaces,
            spaces_to,
            remove_accents,
            remove_special,
            ascii_only,
            remove_brackets,
            fix_invalid,
        } => {
            egui::Grid::new(id).num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                ui.checkbox(trim, "Rogner les bords");
                ui.checkbox(collapse_spaces, "Espaces multiples");
                ui.end_row();
                ui.checkbox(separators_to_spaces, "_ et . en espaces");
                ui.checkbox(remove_accents, "Retirer les accents");
                ui.end_row();
                ui.checkbox(remove_special, "Caractères spéciaux");
                ui.checkbox(ascii_only, "ASCII uniquement");
                ui.end_row();
                ui.checkbox(remove_brackets, "(…) […] {…}");
                ui.checkbox(fix_invalid, "Caractères interdits");
                ui.end_row();
            });
            ui.horizontal(|ui| {
                row_label(ui, p, "Espaces remplacés par");
                ui.add(egui::TextEdit::singleline(spaces_to).hint_text("garder").desired_width(70.0));
            });
        }
        Rule::Extension { mode, value } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Action");
                enum_combo(ui, id.with("ext"), mode, &ExtMode::ALL, ExtMode::label, 170.0);
                ui.end_row();
                if matches!(mode, ExtMode::Set | ExtMode::Append) {
                    row_label(ui, p, "Extension");
                    text_field(ui, value, "ex : jpg");
                    ui.end_row();
                }
            });
        }
        Rule::Template { pattern } => {
            field_with_tokens(ui, pattern, "{name}_{n:3}");
            ui.label(RichText::new("Remplace tout le nom (l'extension est conservée). Cliquez sur {} pour les balises.").small().color(p.weak));
        }
        Rule::Truncate { max, keep_end } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Longueur max");
                ui.add(egui::DragValue::new(max).range(1..=255));
                ui.end_row();
            });
            ui.checkbox(keep_end, "Garder la fin plutôt que le début");
        }
        Rule::Swap { separator } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Séparateur");
                text_field(ui, separator, " - ");
                ui.end_row();
            });
        }
        Rule::PadNumbers { width } => {
            grid(ui, id, |ui| {
                row_label(ui, p, "Chiffres");
                ui.add(egui::DragValue::new(width).range(1..=12));
                ui.end_row();
            });
        }
    }
}
