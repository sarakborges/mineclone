use std::ops::Range;

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    text::{EditableText, TextEdit},
};

use crate::{
    content::{
        biome::BiomeRegistry, creature::CreatureRegistry, structure::StructureRegistry,
        structure_set::StructureSetRegistry,
    },
    localization::ActiveLanguage,
};

use super::{ChatState, MAX_INPUT_CHARS, visual::ChatDraft};

#[derive(Clone, Copy)]
enum ParameterKind {
    CreatureId,
    MetaTag,
    ModifyAction,
    MetaValue,
    StructureLiteral,
    StructureId,
    StructureVariation,
    LocateKind,
    LocateTargetId,
    Coordinate,
}

struct CommandDefinition {
    name: &'static str,
    description: &'static str,
    parameters: &'static [ParameterKind],
}

const COMMANDS: &[CommandDefinition] = &[
    CommandDefinition {
        name: "spawn",
        description: "Spawn a creature",
        parameters: &[ParameterKind::CreatureId, ParameterKind::MetaTag],
    },
    CommandDefinition {
        name: "kill",
        description: "Kill the targeted creature",
        parameters: &[],
    },
    CommandDefinition {
        name: "modify",
        description: "Modify metadata on the targeted creature",
        parameters: &[
            ParameterKind::ModifyAction,
            ParameterKind::MetaTag,
            ParameterKind::MetaValue,
        ],
    },
    CommandDefinition {
        name: "place",
        description: "Place a structure",
        parameters: &[
            ParameterKind::StructureLiteral,
            ParameterKind::StructureId,
            ParameterKind::StructureVariation,
        ],
    },
    CommandDefinition {
        name: "locate",
        description: "Locate a biome or structure",
        parameters: &[
            ParameterKind::LocateKind,
            ParameterKind::LocateTargetId,
            ParameterKind::StructureVariation,
        ],
    },
    CommandDefinition {
        name: "warp",
        description: "Warp near world coordinates",
        parameters: &[
            ParameterKind::Coordinate,
            ParameterKind::Coordinate,
            ParameterKind::Coordinate,
        ],
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Suggestion {
    pub(super) value: String,
    pub(super) description: String,
}

#[derive(Resource, Default)]
pub(super) struct ChatAutocomplete {
    pub(super) suggestions: Vec<Suggestion>,
    pub(super) selected: usize,
    pub(super) revision: u64,
    replace: Range<usize>,
    last_context: Option<(String, usize)>,
    dismissed: Option<(String, usize)>,
}

impl ChatAutocomplete {
    pub(super) fn visible(&self) -> bool {
        !self.suggestions.is_empty()
    }

    pub(super) fn dismiss(&mut self, editor: &EditableText) {
        self.dismissed = Some(editor_context(editor));
        self.set_suggestions(0..0, Vec::new());
    }

    fn set_suggestions(&mut self, replace: Range<usize>, suggestions: Vec<Suggestion>) {
        if self.replace != replace || self.suggestions != suggestions {
            self.replace = replace;
            self.suggestions = suggestions;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    fn refresh(&mut self, editor: &EditableText, catalog: &AutocompleteCatalog<'_>) {
        if editor.is_composing() {
            self.set_suggestions(0..0, Vec::new());
            return;
        }
        let context = editor_context(editor);
        if self.last_context.as_ref() != Some(&context) {
            self.selected = 0;
            self.last_context = Some(context.clone());
            if self.dismissed.as_ref() != Some(&context) {
                self.dismissed = None;
            }
        }
        if self.dismissed.as_ref() == Some(&context) {
            self.set_suggestions(0..0, Vec::new());
            return;
        }
        let (range, suggestions) =
            suggestions_for(&context.0, context.1, catalog).unwrap_or_else(|| (0..0, Vec::new()));
        self.set_suggestions(range, suggestions);
    }
}

fn editor_context(editor: &EditableText) -> (String, usize) {
    let text = editor.value().to_string();
    let position = editor.editor().raw_selection().focus().index().min(text.len());
    let cursor = if text.is_char_boundary(position) {
        position
    } else {
        text.floor_char_boundary(position)
    };
    (text, cursor)
}

/// The token under the caret, not necessarily the last argument in the line.
fn active_token(text: &str, cursor: usize) -> Option<(Range<usize>, usize)> {
    if !text.starts_with('/') || !text.is_char_boundary(cursor) || cursor > text.len() {
        return None;
    }
    let start = text[..cursor]
        .char_indices()
        .rfind(|(_, character)| character.is_whitespace())
        .map_or(0, |(index, character)| index + character.len_utf8());
    let end = text[cursor..]
        .find(char::is_whitespace)
        .map_or(text.len(), |offset| cursor + offset);
    let parameter = if start == 0 {
        0
    } else {
        text[..start].split_whitespace().count()
    };
    Some((start..end, parameter))
}

fn text_matches_query(value: &str, query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    value.to_ascii_lowercase().contains(&query)
}

fn id_matches_query(id: &str, query: &str) -> bool {
    text_matches_query(id, query)
}

fn literal_suggestions(
    values: &[(&str, &str)],
    prefix: &str,
) -> Vec<Suggestion> {
    values
        .iter()
        .filter(|(value, _)| text_matches_query(value, prefix))
        .map(|(value, description)| Suggestion {
            value: (*value).to_owned(),
            description: (*description).to_owned(),
        })
        .collect()
}

struct AutocompleteCatalog<'a> {
    creatures: &'a CreatureRegistry,
    biomes: &'a BiomeRegistry,
    structures: &'a StructureRegistry,
    structure_sets: &'a StructureSetRegistry,
    language: &'a ActiveLanguage,
}

impl AutocompleteCatalog<'_> {
    fn structure_suggestions(&self, prefix: &str, locatable_only: bool) -> Vec<Suggestion> {
        let mut values = self
            .structures
            .iter()
            .filter(|structure| structure.group_id.is_none())
            .filter(|structure| !locatable_only || structure.locatable)
            .filter(|structure| id_matches_query(&structure.id, prefix))
            .map(|structure| Suggestion {
                value: structure.id.clone(),
                description: structure.name.text(self.language.get()).to_owned(),
            })
            .collect::<Vec<_>>();

        values.extend(
            self.structures
                .group_references()
                .filter(|(_, structure, _)| !locatable_only || structure.locatable)
                .filter(|(reference, _, _)| id_matches_query(reference, prefix))
                .map(|(reference, structure, count)| Suggestion {
                    value: reference.to_owned(),
                    description: format!(
                        "{} ({count} variations)",
                        structure.name.text(self.language.get())
                    ),
                }),
        );
        values.extend(
            self.structure_sets
                .iter()
                .filter(|set| !locatable_only || set.locatable)
                .filter(|set| id_matches_query(&set.id, prefix))
                .map(|set| Suggestion {
                    value: set.id.clone(),
                    description: format!(
                        "{} (structure set)",
                        set.name.text(self.language.get())
                    ),
                }),
        );

        values
    }
}

fn suggestions_for(
    text: &str,
    cursor: usize,
    catalog: &AutocompleteCatalog<'_>,
) -> Option<(Range<usize>, Vec<Suggestion>)> {
    let (range, word_index) = active_token(text, cursor)?;
    let prefix = text[range.start..cursor].to_ascii_lowercase();
    let mut suggestions = if word_index == 0 {
        COMMANDS
            .iter()
            .filter(|command| {
                text_matches_query(command.name, prefix.strip_prefix('/').unwrap_or(&prefix))
            })
            .map(|command| Suggestion {
                value: format!("/{}", command.name),
                description: command.description.to_owned(),
            })
            .collect::<Vec<_>>()
    } else {
        let mut words = text.split_whitespace();
        let command = words.next()?;
        let first_argument = words.next();
        let second_argument = words.next();
        let definition = COMMANDS
            .iter()
            .find(|item| command.strip_prefix('/') == Some(item.name))?;
        match definition.parameters.get(word_index - 1)? {
            ParameterKind::StructureLiteral => literal_suggestions(
                &[("structure", "Structure")],
                &prefix,
            ),
            ParameterKind::CreatureId => catalog
                .creatures
                .iter()
                .filter(|creature| id_matches_query(&creature.id, &prefix))
                .map(|creature| Suggestion {
                    value: creature.id.clone(),
                    description: creature.name.text(catalog.language.get()).to_owned(),
                })
                .collect::<Vec<_>>(),
            ParameterKind::MetaTag => literal_suggestions(
                &[("NO_AI", "Freeze entity AI, damage and collision")],
                &prefix,
            ),
            ParameterKind::ModifyAction => literal_suggestions(
                &[
                    ("add", "Add a metadata tag"),
                    ("remove", "Remove a metadata tag"),
                    ("edit", "Edit a metadata tag value"),
                ],
                &prefix,
            ),
            ParameterKind::MetaValue => Vec::new(),
            ParameterKind::StructureId => catalog.structure_suggestions(&prefix, false),
            ParameterKind::StructureVariation => {
                if first_argument? != "structure" {
                    return Some((range, Vec::new()));
                }
                let reference = second_argument?;
                if catalog.structure_sets.get(reference).is_some() {
                    Vec::new()
                } else {
                    let count = catalog.structures.variation_count(reference)?;
                    (1..=count)
                        .filter(|variation| text_matches_query(&variation.to_string(), &prefix))
                        .filter_map(|variation| {
                            let structure = catalog.structures.variation(reference, variation)?;
                            Some(Suggestion {
                                value: variation.to_string(),
                                description: structure.id.clone(),
                            })
                        })
                        .collect::<Vec<_>>()
                }
            }
            ParameterKind::LocateKind => literal_suggestions(
                &[
                    ("biome", "Locate a surface or volume biome"),
                    ("structure", "Locate a locatable structure"),
                ],
                &prefix,
            ),
            ParameterKind::LocateTargetId => match first_argument? {
                "biome" => catalog
                    .biomes
                    .iter()
                    .filter(|biome| id_matches_query(&biome.id, &prefix))
                    .map(|biome| Suggestion {
                        value: biome.id.clone(),
                        description: biome.name.text(catalog.language.get()).to_owned(),
                    })
                    .collect::<Vec<_>>(),
                "structure" => catalog.structure_suggestions(&prefix, true),
                _ => Vec::new(),
            },
            ParameterKind::Coordinate => Vec::new(),
        }
    };
    suggestions.sort_by(|left, right| left.value.cmp(&right.value));
    Some((range, suggestions))
}

fn completed_line(text: &str, range: Range<usize>, value: &str) -> (String, usize) {
    let suffix = &text[range.end..];
    let append_space = suffix.chars().next().is_none_or(|character| !character.is_whitespace());
    let replacement = if append_space {
        format!("{value} ")
    } else {
        value.to_owned()
    };
    let mut result = String::with_capacity(text.len() + replacement.len());
    result.push_str(&text[..range.start]);
    result.push_str(&replacement);
    let caret = result.len();
    result.push_str(suffix);
    (result, caret)
}

#[derive(SystemParam)]
pub(super) struct AutocompleteContent<'w> {
    creatures: Res<'w, CreatureRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    structures: Res<'w, StructureRegistry>,
    structure_sets: Res<'w, StructureSetRegistry>,
    language: Res<'w, ActiveLanguage>,
}

impl AutocompleteContent<'_> {
    fn catalog(&self) -> AutocompleteCatalog<'_> {
        AutocompleteCatalog {
            creatures: &self.creatures,
            biomes: &self.biomes,
            structures: &self.structures,
            structure_sets: &self.structure_sets,
            language: &self.language,
        }
    }
}

/// Called after chat opening/closing; never steals focus or submits a command.
pub(super) fn update_autocomplete(
    chat: Res<ChatState>,
    keys: Res<ButtonInput<KeyCode>>,
    content: AutocompleteContent,
    mut autocomplete: ResMut<ChatAutocomplete>,
    mut draft: Single<&mut EditableText, With<ChatDraft>>,
) {
    if !chat.is_open() {
        if autocomplete.visible() {
            autocomplete.set_suggestions(0..0, Vec::new());
        }
        return;
    }
    let catalog = content.catalog();
    autocomplete.refresh(&draft, &catalog);
    if !autocomplete.visible() || draft.is_composing() {
        return;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        autocomplete.selected = (autocomplete.selected + autocomplete.suggestions.len() - 1)
            % autocomplete.suggestions.len();
        autocomplete.revision = autocomplete.revision.wrapping_add(1);
        draft.pending_edits.retain(|edit| !matches!(edit, TextEdit::Up(_)));
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        autocomplete.selected = (autocomplete.selected + 1) % autocomplete.suggestions.len();
        autocomplete.revision = autocomplete.revision.wrapping_add(1);
        draft.pending_edits.retain(|edit| !matches!(edit, TextEdit::Down(_)));
    } else if keys.just_pressed(KeyCode::Tab) {
        let (text, _) = editor_context(&draft);
        let selected = &autocomplete.suggestions[autocomplete.selected].value;
        let (completed, caret) = completed_line(&text, autocomplete.replace.clone(), selected);
        if completed.chars().count() > MAX_INPUT_CHARS {
            return;
        }
        let suffix_characters = completed[caret..].chars().count();
        draft.pending_edits.clear();
        draft.editor_mut().set_text(&completed);
        draft.queue_edit(TextEdit::TextEnd(false));
        for _ in 0..suffix_characters {
            draft.queue_edit(TextEdit::Left(false));
        }
        autocomplete.set_suggestions(0..0, Vec::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_catalog_contains_entity_commands() {
        assert!(COMMANDS.iter().any(|command| command.name == "kill"));
        assert!(COMMANDS.iter().any(|command| command.name == "modify"));
    }

    #[test]
    fn autocomplete_matching_accepts_substrings_anywhere() {
        assert!(text_matches_query("warp", "ar"));
        assert!(text_matches_query("structure", "ruc"));
        assert!(id_matches_query("asteria:world_tree", "world"));
        assert!(id_matches_query("asteria:world_tree", "tree"));
        assert!(id_matches_query("asteria:world_tree", "STERIA"));
        assert!(!id_matches_query("asteria:world_tree", "slime"));
    }

    #[test]
    fn token_detection_handles_partial_parameters_and_caret_in_middle() {
        assert_eq!(active_token("/", 1), Some((0..1, 0)));
        assert_eq!(active_token("/spawn ", 7), Some((7..7, 1)));
        assert_eq!(active_token("/spawn id", 8), Some((7..9, 1)));
        assert_eq!(active_token("hello", 5), None);
    }

    #[test]
    fn tab_completion_always_advances_to_the_next_argument() {
        assert_eq!(
            completed_line("/spa", 0..4, "/spawn"),
            ("/spawn ".to_owned(), 7)
        );
        let text = "/spawn me tail";
        let expected = "/spawn asteria:meadow_slime tail";
        assert_eq!(
            completed_line(text, 7..9, "asteria:meadow_slime"),
            (expected.to_owned(), "/spawn asteria:meadow_slime".len())
        );
    }
}
