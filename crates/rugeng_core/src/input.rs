//! Project Ember — Gelişmiş Girdi Eşleme Sistemi (Input Action Mapping).
//!
//! Fiziksel donanım tuşlarını (Keyboard KeyCode / Mouse Buttons) anlamsal eylemlere (Actions) bağlar.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionInput {
    Key(u32),        // Fiziksel tuş tarama kodu
    MouseButton(u8), // 0: Sol, 1: Sağ, 2: Orta
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ActionMap {
    bindings: HashMap<String, Vec<ActionInput>>,
    #[serde(skip)]
    currently_pressed: HashSet<ActionInput>,
    #[serde(skip)]
    just_pressed: HashSet<ActionInput>,
    #[serde(skip)]
    just_released: HashSet<ActionInput>,
}

impl Default for ActionMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionMap {
    pub fn new() -> Self {
        let mut map = Self {
            bindings: HashMap::new(),
            currently_pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
        };
        map.setup_default_bindings();
        map
    }

    pub fn setup_default_bindings(&mut self) {
        // Standart Godot / Ember eşlemeleri
        self.bind("move_left", ActionInput::Key(0x1E)); // A
        self.bind("move_left", ActionInput::Key(0xCB)); // Sol Ok
        self.bind("move_right", ActionInput::Key(0x20)); // D
        self.bind("move_right", ActionInput::Key(0xCD)); // Sağ Ok
        self.bind("move_up", ActionInput::Key(0x11)); // W
        self.bind("move_up", ActionInput::Key(0xC8)); // Yukarı Ok
        self.bind("move_down", ActionInput::Key(0x1F)); // S
        self.bind("move_down", ActionInput::Key(0xD0)); // Aşağı Ok
        self.bind("action_fire", ActionInput::Key(0x39)); // Boşluk
        self.bind("action_fire", ActionInput::MouseButton(0)); // Sol Tık
        self.bind("action_alt", ActionInput::MouseButton(1)); // Sağ Tık
        self.bind("toggle_grid", ActionInput::Key(0x1F)); // S
        self.bind("focus_camera", ActionInput::Key(0x21)); // F
    }

    pub fn bind(&mut self, action: &str, input: ActionInput) {
        let entry = self.bindings.entry(action.to_string()).or_default();
        if !entry.contains(&input) {
            entry.push(input);
        }
    }

    /// Bir eylemin tüm tuş atamalarını sıfırlayıp tek bir yeni tuşa bağlar (Remap).
    pub fn remap(&mut self, action: &str, input: ActionInput) {
        self.bindings.insert(action.to_string(), vec![input]);
    }

    /// Belirtilen eylemden tek bir girdiyi kaldırır.
    pub fn unbind(&mut self, action: &str, input: ActionInput) {
        if let Some(inputs) = self.bindings.get_mut(action) {
            inputs.retain(|&i| i != input);
        }
    }

    /// Bir eylemin tüm girdilerini temizler.
    pub fn clear_action(&mut self, action: &str) {
        self.bindings.remove(action);
    }

    /// Bir eyleme bağlı tüm girdileri döndürür.
    pub fn get_bindings(&self, action: &str) -> Option<&[ActionInput]> {
        self.bindings.get(action).map(|v| v.as_slice())
    }

    /// Tüm eylemlerin haritasına referans verir.
    pub fn all_bindings(&self) -> &HashMap<String, Vec<ActionInput>> {
        &self.bindings
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.bindings)
            .map_err(|e| format!("Input JSON serilestirme hatasi: {e}"))
    }

    pub fn from_json(&mut self, json_str: &str) -> Result<(), String> {
        let loaded: HashMap<String, Vec<ActionInput>> =
            serde_json::from_str(json_str).map_err(|e| format!("Input JSON cozme hatasi: {e}"))?;
        self.bindings = loaded;
        Ok(())
    }

    pub fn update_key_event(&mut self, code: u32, pressed: bool) {
        let input = ActionInput::Key(code);
        if pressed {
            if self.currently_pressed.insert(input) {
                self.just_pressed.insert(input);
            }
        } else if self.currently_pressed.remove(&input) {
            self.just_released.insert(input);
        }
    }

    pub fn update_mouse_button(&mut self, btn: u8, pressed: bool) {
        let input = ActionInput::MouseButton(btn);
        if pressed {
            if self.currently_pressed.insert(input) {
                self.just_pressed.insert(input);
            }
        } else if self.currently_pressed.remove(&input) {
            self.just_released.insert(input);
        }
    }

    pub fn is_action_pressed(&self, action: &str) -> bool {
        if let Some(inputs) = self.bindings.get(action) {
            inputs
                .iter()
                .any(|inp| self.currently_pressed.contains(inp))
        } else {
            false
        }
    }

    pub fn is_action_just_pressed(&self, action: &str) -> bool {
        if let Some(inputs) = self.bindings.get(action) {
            inputs.iter().any(|inp| self.just_pressed.contains(inp))
        } else {
            false
        }
    }

    pub fn end_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}
