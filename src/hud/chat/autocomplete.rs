use std::ops::Range;

use bevy::{
    prelude::*,
    text::{EditableText, TextEdit},
};

use crate::{
    content::{creature::CreatureRegistry, structure::StructureRegistry},
    localization::ActiveLanguage,
};

use super::{ChatState, MAX_INPUT_CHARS, visual::ChatDraft};

/// Command signatures are the single source of truth for parsing, help and completion.
#[derive(Clone, Copy)]
enum CommandId {
    Spawn,
    Place,
}

#[derive(Clone, Copy)]
enum ParameterKind {
    CreatureId,
    StructureId,
}

struct CommandDefinition {
    name: &'static str,
    usage: &'static str,
    description: &'static str,
    parameters: &'static [ParameterKind],
    id: CommandId,
}

const COMMANDS: &[CommandDefinition] = &[
    CommandDefinition {
        name: "spawn",
        usage: "/spawn <id>",
        description: "Spawn a creature",
        parameters: &[ParameterKind::CreatureId],
        id: CommandId::Spawn,
    },
    CommandDefinition {
        name: "place",
        usage: "/place <id>",
        description: "Place a structure",
        parameters: &[ParameterKind::StructureId],
        id: CommandId::Place,
    },
];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ParsedLine<'a> {
    Say(&'a str),
    Spawn(&'a str),
    Place(&'a str),
    Usage(&'static str),
    Unknown(&'a str),
}

pub(super) fn parse_line(input: &str) -> ParsedLine<'_> {
    let line = input.trim();
    if !line.starts_with('/') {
        return ParsedLine::Say(line);
    }
    let mut words = line.split_whitespace();
    let Some(name) = words.next() else {
        return ParsedLine::Unknown("/");
    };
    let Some(definition) = COMMANDS
        .iter()
        .find(|definition| name.strip_prefix('/') == Some(definition.name))
    else {
        return ParsedLine::Unknown(name);
    };
    let args: Vec<_> = words.collect();
    if args.len() != definition.parameters.len() {
        return ParsedLine::Usage(definition.usage);
    }
    match definition.id {
        CommandId::Spawn => ParsedLine::Spawn(args[0]),
        CommandId::Place => ParsedLine::Place(args[0]),
    }
}

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

    fn refresh(
        &mut self,
        editor: &EditableText,
        creatures: &CreatureRegistry,
        structures: &StructureRegistry,
        language: &ActiveLanguage,
    ) {
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
        let (range, suggestions) = suggestions_for(
            &context.0,
            context.1,
            creatures,
            structures,
            language,
        )
        .unwrap_or_else(|| (0..0, Vec::new()));
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

fn suggestions_for(
    text: &str,
    cursor: usize,
    creatures: &CreatureRegistry,
    structures: &StructureRegistry,
    language: &ActiveLanguage,
) -> Option<(Range<usize>, Vec<Suggestion>)> {
    let (range, word_index) = active_token(text, cursor)?;
    let prefix = text[range.start..cursor].to_ascii_lowercase();
    let mut suggestions = if word_index == 0 {
        COMMANDS
            .iter()
            .filter(|command| format!("/{}", command.name).starts_with(&prefix))
            .map(|command| Suggestion {
                value: format!("/{}", command.name),
                description: command.description.to_owned(),
            })
            .collect::<Vec<_>>()
    } else {
        let command = text.split_whitespace().next()?;
        let definition = COMMANDS
            .iter()
            .find(|item| command.strip_prefix('/') == Some(item.name))?;
        match definition.parameters.get(word_index - 1)? {
            ParameterKind::CreatureId => creatures
                .iter()
                .filter(|creature| {
                    let id = creature.id.to_ascii_lowercase();
                    id.starts_with(&prefix)
                        || id.strip_prefix("asteria:").is_some_and(|short| short.starts_with(&prefix))
                })
                .map(|creature| Suggestion {
                    value: creature.id.clone(),
                    description: creature.name.text(language.get()).to_owned(),
                })
                .collect::<Vec<_>>(),
            ParameterKind::StructureId => structures
                .iter()
                .filter(|structure| {
                    let id = structure.id.to_ascii_lowercase();
                    id.starts_with(&prefix)
                        || id.strip_prefix("asteria:").is_some_and(|short| short.starts_with(&prefix))
                })
                .map(|structure| Suggestion {
                    value: structure.id.clone(),
                    description: structure.name.text(language.get()).to_owned(),
                })
                .collect::<Vec<_>>(),
        }
    };
    suggestions.sort_by(|left, right| left.value.cmp(&right.value));
    Some((range, suggestions))
}

fn completed_line(text: &str, range: Range<usize>, value: &str) -> (String, usize) {
    let command = range.start == 0;
    let append_space = command && text[range.end..].chars().next().is_none_or(|c| !c.is_whitespace());
    let replacement = if append_space { format!("{value} ") } else { value.to_owned() };
    let mut result = String::with_capacity(text.len() + replacement.len());
    result.push_str(&text[..range.start]);
    result.push_str(&replacement);
    let caret = result.len();
    result.push_str(&text[range.end..]);
    (result, caret)
}

/// Called after chat opening/closing; never steals focus or submits a command.
pub(super) fn update_autocomplete(
    chat: Res<ChatState>,
    keys: Res<ButtonInput<KeyCode>>,
    creatures: Res<CreatureRegistry>,
    structures: Res<StructureRegistry>,
    language: Res<ActiveLanguage>,
    mut autocomplete: ResMut<ChatAutocomplete>,
    mut draft: Single<&mut EditableText, With<ChatDraft>>,
) {
    if !chat.is_open() {
        if autocomplete.visible() {
            autocomplete.set_suggestions(0..0, Vec::new());
        }
        return;
    }
    autocomplete.refresh(&draft, &creatures, &structures, &language);
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
        // Preserve other arguments when completing a token in the middle of a line.
        // These edits put the caret immediately after the replacement.
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
    fn command_registry_also_drives_parser() {
        assert_eq!(parse_line("hello"), ParsedLine::Say("hello"));
        assert_eq!(parse_line("/spawn asteria:meadow_slime"), ParsedLine::Spawn("asteria:meadow_slime"));
        assert_eq!(parse_line("/place asteria:hut"), ParsedLine::Place("asteria:hut"));
        assert_eq!(parse_line("/spawn"), ParsedLine::Usage("/spawn <id>"));
        assert_eq!(parse_line("/place extra extra"), ParsedLine::Usage("/place <id>"));
        assert_eq!(parse_line("/spawn_creature old"), ParsedLine::Unknown("/spawn_creature"));
        assert_eq!(parse_line("/missing"), ParsedLine::Unknown("/missing"));
    }

    #[test]
    fn token_detection_handles_partial_parameters_and_caret_in_middle() {
        assert_eq!(active_token("/", 1), Some((0..1, 0)));
        assert_eq!(active_token("/spawn ", 7), Some((7..7, 1)));
        assert_eq!(active_token("/spawn id", 8), Some((7..9, 1)));
        assert_eq!(active_token("hello", 5), None);
    }

    #[test]
    fn completing_command_adds_space_and_preserves_existing_arguments() {
        assert_eq!(completed_line("/spa", 0..4, "/spawn"), ("/spawn ".to_owned(), 7));
        let text = "/spawn me tail";
        let expected = "/spawn asteria:meadow_slime tail";
        assert_eq!(
            completed_line(text, 7..9, "asteria:meadow_slime"),
            (expected.to_owned(), "/spawn asteria:meadow_slime".len())
        );
    }
}
