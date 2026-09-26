use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeybindAction {
    Jump,
    Descend,
    Inventory,
    Chat,
    ToolAction,
    DropItem,
    ChangePerspective,
}

impl KeybindAction {
    pub(crate) const ALL: [Self; 7] = [
        Self::Jump,
        Self::Descend,
        Self::Inventory,
        Self::Chat,
        Self::ToolAction,
        Self::DropItem,
        Self::ChangePerspective,
    ];

    pub(crate) const fn localization_key(self) -> &'static str {
        match self {
            Self::Jump => "settings.keybind.jump",
            Self::Descend => "settings.keybind.descend",
            Self::Inventory => "settings.keybind.inventory",
            Self::Chat => "settings.keybind.chat",
            Self::ToolAction => "settings.keybind.toolAction",
            Self::DropItem => "settings.keybind.dropItem",
            Self::ChangePerspective => "settings.keybind.changePerspective",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KeyboardKey {
    Space,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
}

impl KeyboardKey {
    pub(crate) const fn key_code(self) -> KeyCode {
        match self {
            Self::Space => KeyCode::Space,
            Self::ShiftLeft => KeyCode::ShiftLeft,
            Self::ShiftRight => KeyCode::ShiftRight,
            Self::ControlLeft => KeyCode::ControlLeft,
            Self::ControlRight => KeyCode::ControlRight,
            Self::AltLeft => KeyCode::AltLeft,
            Self::AltRight => KeyCode::AltRight,
            Self::Tab => KeyCode::Tab,
            Self::Backspace => KeyCode::Backspace,
            Self::Delete => KeyCode::Delete,
            Self::Insert => KeyCode::Insert,
            Self::Home => KeyCode::Home,
            Self::End => KeyCode::End,
            Self::PageUp => KeyCode::PageUp,
            Self::PageDown => KeyCode::PageDown,
            Self::ArrowUp => KeyCode::ArrowUp,
            Self::ArrowDown => KeyCode::ArrowDown,
            Self::ArrowLeft => KeyCode::ArrowLeft,
            Self::ArrowRight => KeyCode::ArrowRight,
            Self::F1 => KeyCode::F1,
            Self::F2 => KeyCode::F2,
            Self::F3 => KeyCode::F3,
            Self::F4 => KeyCode::F4,
            Self::F5 => KeyCode::F5,
            Self::F6 => KeyCode::F6,
            Self::F7 => KeyCode::F7,
            Self::F8 => KeyCode::F8,
            Self::F9 => KeyCode::F9,
            Self::F10 => KeyCode::F10,
            Self::F11 => KeyCode::F11,
            Self::F12 => KeyCode::F12,
            Self::Digit0 => KeyCode::Digit0,
            Self::Digit1 => KeyCode::Digit1,
            Self::Digit2 => KeyCode::Digit2,
            Self::Digit3 => KeyCode::Digit3,
            Self::Digit4 => KeyCode::Digit4,
            Self::Digit5 => KeyCode::Digit5,
            Self::Digit6 => KeyCode::Digit6,
            Self::Digit7 => KeyCode::Digit7,
            Self::Digit8 => KeyCode::Digit8,
            Self::Digit9 => KeyCode::Digit9,
            Self::KeyA => KeyCode::KeyA,
            Self::KeyB => KeyCode::KeyB,
            Self::KeyC => KeyCode::KeyC,
            Self::KeyD => KeyCode::KeyD,
            Self::KeyE => KeyCode::KeyE,
            Self::KeyF => KeyCode::KeyF,
            Self::KeyG => KeyCode::KeyG,
            Self::KeyH => KeyCode::KeyH,
            Self::KeyI => KeyCode::KeyI,
            Self::KeyJ => KeyCode::KeyJ,
            Self::KeyK => KeyCode::KeyK,
            Self::KeyL => KeyCode::KeyL,
            Self::KeyM => KeyCode::KeyM,
            Self::KeyN => KeyCode::KeyN,
            Self::KeyO => KeyCode::KeyO,
            Self::KeyP => KeyCode::KeyP,
            Self::KeyQ => KeyCode::KeyQ,
            Self::KeyR => KeyCode::KeyR,
            Self::KeyS => KeyCode::KeyS,
            Self::KeyT => KeyCode::KeyT,
            Self::KeyU => KeyCode::KeyU,
            Self::KeyV => KeyCode::KeyV,
            Self::KeyW => KeyCode::KeyW,
            Self::KeyX => KeyCode::KeyX,
            Self::KeyY => KeyCode::KeyY,
            Self::KeyZ => KeyCode::KeyZ,
        }
    }

    pub(crate) const fn from_key_code(key: KeyCode) -> Option<Self> {
        Some(match key {
            KeyCode::Space => Self::Space,
            KeyCode::ShiftLeft => Self::ShiftLeft,
            KeyCode::ShiftRight => Self::ShiftRight,
            KeyCode::ControlLeft => Self::ControlLeft,
            KeyCode::ControlRight => Self::ControlRight,
            KeyCode::AltLeft => Self::AltLeft,
            KeyCode::AltRight => Self::AltRight,
            KeyCode::Tab => Self::Tab,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::Delete => Self::Delete,
            KeyCode::Insert => Self::Insert,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,
            KeyCode::ArrowUp => Self::ArrowUp,
            KeyCode::ArrowDown => Self::ArrowDown,
            KeyCode::ArrowLeft => Self::ArrowLeft,
            KeyCode::ArrowRight => Self::ArrowRight,
            KeyCode::F1 => Self::F1,
            KeyCode::F2 => Self::F2,
            KeyCode::F3 => Self::F3,
            KeyCode::F4 => Self::F4,
            KeyCode::F5 => Self::F5,
            KeyCode::F6 => Self::F6,
            KeyCode::F7 => Self::F7,
            KeyCode::F8 => Self::F8,
            KeyCode::F9 => Self::F9,
            KeyCode::F10 => Self::F10,
            KeyCode::F11 => Self::F11,
            KeyCode::F12 => Self::F12,
            KeyCode::Digit0 => Self::Digit0,
            KeyCode::Digit1 => Self::Digit1,
            KeyCode::Digit2 => Self::Digit2,
            KeyCode::Digit3 => Self::Digit3,
            KeyCode::Digit4 => Self::Digit4,
            KeyCode::Digit5 => Self::Digit5,
            KeyCode::Digit6 => Self::Digit6,
            KeyCode::Digit7 => Self::Digit7,
            KeyCode::Digit8 => Self::Digit8,
            KeyCode::Digit9 => Self::Digit9,
            KeyCode::KeyA => Self::KeyA,
            KeyCode::KeyB => Self::KeyB,
            KeyCode::KeyC => Self::KeyC,
            KeyCode::KeyD => Self::KeyD,
            KeyCode::KeyE => Self::KeyE,
            KeyCode::KeyF => Self::KeyF,
            KeyCode::KeyG => Self::KeyG,
            KeyCode::KeyH => Self::KeyH,
            KeyCode::KeyI => Self::KeyI,
            KeyCode::KeyJ => Self::KeyJ,
            KeyCode::KeyK => Self::KeyK,
            KeyCode::KeyL => Self::KeyL,
            KeyCode::KeyM => Self::KeyM,
            KeyCode::KeyN => Self::KeyN,
            KeyCode::KeyO => Self::KeyO,
            KeyCode::KeyP => Self::KeyP,
            KeyCode::KeyQ => Self::KeyQ,
            KeyCode::KeyR => Self::KeyR,
            KeyCode::KeyS => Self::KeyS,
            KeyCode::KeyT => Self::KeyT,
            KeyCode::KeyU => Self::KeyU,
            KeyCode::KeyV => Self::KeyV,
            KeyCode::KeyW => Self::KeyW,
            KeyCode::KeyX => Self::KeyX,
            KeyCode::KeyY => Self::KeyY,
            KeyCode::KeyZ => Self::KeyZ,
            _ => return None,
        })
    }

    pub(crate) const fn is_reserved(self) -> bool {
        matches!(
            self,
            Self::KeyW
                | Self::KeyA
                | Self::KeyS
                | Self::KeyD
                | Self::Digit1
                | Self::Digit2
                | Self::Digit3
                | Self::Digit4
                | Self::Digit5
                | Self::Digit6
                | Self::Digit7
                | Self::Digit8
                | Self::Digit9
        )
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Space => "SPACE",
            Self::ShiftLeft => "L-SHIFT",
            Self::ShiftRight => "R-SHIFT",
            Self::ControlLeft => "L-CTRL",
            Self::ControlRight => "R-CTRL",
            Self::AltLeft => "L-ALT",
            Self::AltRight => "R-ALT",
            Self::Tab => "TAB",
            Self::Backspace => "BACKSPACE",
            Self::Delete => "DELETE",
            Self::Insert => "INSERT",
            Self::Home => "HOME",
            Self::End => "END",
            Self::PageUp => "PAGE UP",
            Self::PageDown => "PAGE DOWN",
            Self::ArrowUp => "↑",
            Self::ArrowDown => "↓",
            Self::ArrowLeft => "←",
            Self::ArrowRight => "→",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::Digit0 => "0",
            Self::Digit1 => "1",
            Self::Digit2 => "2",
            Self::Digit3 => "3",
            Self::Digit4 => "4",
            Self::Digit5 => "5",
            Self::Digit6 => "6",
            Self::Digit7 => "7",
            Self::Digit8 => "8",
            Self::Digit9 => "9",
            Self::KeyA => "A",
            Self::KeyB => "B",
            Self::KeyC => "C",
            Self::KeyD => "D",
            Self::KeyE => "E",
            Self::KeyF => "F",
            Self::KeyG => "G",
            Self::KeyH => "H",
            Self::KeyI => "I",
            Self::KeyJ => "J",
            Self::KeyK => "K",
            Self::KeyL => "L",
            Self::KeyM => "M",
            Self::KeyN => "N",
            Self::KeyO => "O",
            Self::KeyP => "P",
            Self::KeyQ => "Q",
            Self::KeyR => "R",
            Self::KeyS => "S",
            Self::KeyT => "T",
            Self::KeyU => "U",
            Self::KeyV => "V",
            Self::KeyW => "W",
            Self::KeyX => "X",
            Self::KeyY => "Y",
            Self::KeyZ => "Z",
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Keybinds {
    jump: KeyboardKey,
    descend: KeyboardKey,
    inventory: KeyboardKey,
    chat: KeyboardKey,
    tool_action: KeyboardKey,
    drop_item: KeyboardKey,
    change_perspective: KeyboardKey,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            jump: KeyboardKey::Space,
            descend: KeyboardKey::ShiftLeft,
            inventory: KeyboardKey::KeyE,
            chat: KeyboardKey::KeyT,
            tool_action: KeyboardKey::KeyR,
            drop_item: KeyboardKey::KeyQ,
            change_perspective: KeyboardKey::F5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeybindSetError {
    Reserved,
    Conflict(KeybindAction),
}

impl Keybinds {
    pub(crate) const fn key(self, action: KeybindAction) -> KeyboardKey {
        match action {
            KeybindAction::Jump => self.jump,
            KeybindAction::Descend => self.descend,
            KeybindAction::Inventory => self.inventory,
            KeybindAction::Chat => self.chat,
            KeybindAction::ToolAction => self.tool_action,
            KeybindAction::DropItem => self.drop_item,
            KeybindAction::ChangePerspective => self.change_perspective,
        }
    }

    pub(crate) const fn key_code(self, action: KeybindAction) -> KeyCode {
        self.key(action).key_code()
    }

    pub(crate) const fn label(self, action: KeybindAction) -> &'static str {
        self.key(action).label()
    }

    pub(crate) fn set(
        &mut self,
        action: KeybindAction,
        key: KeyboardKey,
    ) -> Result<(), KeybindSetError> {
        if key.is_reserved() {
            return Err(KeybindSetError::Reserved);
        }
        if let Some(conflict) = KeybindAction::ALL
            .into_iter()
            .find(|candidate| *candidate != action && self.key(*candidate) == key)
        {
            return Err(KeybindSetError::Conflict(conflict));
        }

        match action {
            KeybindAction::Jump => self.jump = key,
            KeybindAction::Descend => self.descend = key,
            KeybindAction::Inventory => self.inventory = key,
            KeybindAction::Chat => self.chat = key,
            KeybindAction::ToolAction => self.tool_action = key,
            KeybindAction::DropItem => self.drop_item = key,
            KeybindAction::ChangePerspective => self.change_perspective = key,
        }
        Ok(())
    }
}
