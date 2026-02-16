use std::sync::Arc;

use anyhow::Result;

use crate::config::{AppConfig, MatchBehavior};
use crate::core::expansion::{parse_expansion_actions, OutputAction};
use crate::io::events::{KeyEvent, KeyEventKind, SpecialInputKey};
use crate::io::output::{OutputSink, SpecialKey};

pub struct Engine {
    config: AppConfig,
    output: Option<Arc<dyn OutputSink>>,
    typed_buffer: String,
    max_trigger_chars: usize,
    debug: bool,
}

impl Engine {
    pub fn new(config: AppConfig) -> Self {
        let max_trigger_chars = config
            .expansions
            .iter()
            .map(|r| r.trigger.chars().count())
            .max()
            .unwrap_or(0);

        Self {
            config,
            output: None,
            typed_buffer: String::new(),
            max_trigger_chars,
            debug: false,
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn set_output(&mut self, output: Arc<dyn OutputSink>) {
        self.output = Some(output);
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> Result<()> {
        if event.kind != KeyEventKind::Press {
            return Ok(());
        }

        if event.is_injected {
            return Ok(());
        }

        if let Some(c) = event.printable {
            self.on_printable_char(c)?;
            return Ok(());
        }

        if let Some(key) = event.special {
            self.on_special_key(key)?;
        }

        Ok(())
    }

    fn on_printable_char(&mut self, c: char) -> Result<()> {
        self.typed_buffer.push(c);
        self.truncate_buffer_if_needed();
        self.log_possible_match_buffer();

        match self.config.match_behavior {
            MatchBehavior::Immediate => self.try_expand_immediate()?,
            MatchBehavior::Boundary => {
                if self.is_boundary_char(c) {
                    self.try_expand_boundary(Some(c), None)?;
                }
            }
        }

        Ok(())
    }

    fn log_possible_match_buffer(&self) {
        if !self.debug {
            return;
        }

        if self.find_possible_trigger_suffix().is_some() {
            eprintln!("possible match buffer: {:?}", self.typed_buffer);
        }
    }

    fn find_possible_trigger_suffix(&self) -> Option<&str> {
        for (start, _) in self.typed_buffer.char_indices() {
            let suffix = &self.typed_buffer[start..];
            for rule in &self.config.expansions {
                if rule.trigger.starts_with(suffix) {
                    return Some(suffix);
                }
            }
        }
        None
    }

    fn on_special_key(&mut self, key: SpecialInputKey) -> Result<()> {
        match key {
            SpecialInputKey::Backspace => {
                self.typed_buffer.pop();
            }
            SpecialInputKey::Shift
            | SpecialInputKey::Ctrl
            | SpecialInputKey::Alt
            | SpecialInputKey::Meta
            | SpecialInputKey::CapsLock => {}
            SpecialInputKey::Enter | SpecialInputKey::Tab => {
                if self.config.match_behavior == MatchBehavior::Boundary {
                    self.try_expand_boundary(None, Some(key))?;
                } else {
                    self.typed_buffer.clear();
                }
            }
            _ => {
                self.typed_buffer.clear();
            }
        }
        Ok(())
    }

    fn try_expand_immediate(&mut self) -> Result<()> {
        for rule in &self.config.expansions {
            if self.typed_buffer.ends_with(&rule.trigger) {
                eprintln!(
                    "trigger detected (immediate): '{}' -> expansion fired",
                    rule.trigger
                );
                let mut actions = parse_expansion_actions(&rule.expansion)?;
                self.execute_expansion(rule.trigger.chars().count(), &mut actions)?;
                break;
            }
        }
        Ok(())
    }

    fn try_expand_boundary(
        &mut self,
        typed_boundary_char: Option<char>,
        typed_boundary_key: Option<SpecialInputKey>,
    ) -> Result<()> {
        let mut candidate = self.typed_buffer.clone();
        if typed_boundary_char.is_some() {
            candidate.pop();
        }

        for rule in &self.config.expansions {
            if candidate.ends_with(&rule.trigger) {
                let boundary = if let Some(c) = typed_boundary_char {
                    format!("char '{}'", c)
                } else if let Some(key) = typed_boundary_key {
                    format!("key {:?}", key)
                } else {
                    "none".to_string()
                };
                eprintln!(
                    "trigger detected (boundary): '{}' at {} -> expansion fired",
                    rule.trigger, boundary
                );
                let mut actions = parse_expansion_actions(&rule.expansion)?;
                if let Some(c) = typed_boundary_char {
                    actions.push(OutputAction::Text(c.to_string()));
                }
                if let Some(key) = typed_boundary_key {
                    if let Some(mapped) = map_input_key_to_output_key(key) {
                        actions.push(OutputAction::Key(mapped));
                    }
                }

                let delete_count = rule.trigger.chars().count()
                    + usize::from(typed_boundary_char.is_some() || typed_boundary_key.is_some());
                self.execute_expansion(delete_count, &mut actions)?;
                break;
            }
        }

        Ok(())
    }

    fn execute_expansion(
        &mut self,
        backspaces: usize,
        actions: &mut [OutputAction],
    ) -> Result<()> {
        if let Some(output) = &self.output {
            output.send_backspaces(backspaces)?;
            output.send_actions(actions)?;
        }
        self.typed_buffer.clear();
        Ok(())
    }

    fn truncate_buffer_if_needed(&mut self) {
        let max_len = self.max_trigger_chars.saturating_add(8);
        if self.typed_buffer.chars().count() <= max_len {
            return;
        }

        let keep_from = self.typed_buffer.chars().count().saturating_sub(max_len);
        self.typed_buffer = self.typed_buffer.chars().skip(keep_from).collect();
    }

    fn is_boundary_char(&self, c: char) -> bool {
        self.config.boundary_chars().contains(c)
    }
}

fn map_input_key_to_output_key(key: SpecialInputKey) -> Option<SpecialKey> {
    match key {
        SpecialInputKey::Enter => Some(SpecialKey::Enter),
        SpecialInputKey::Tab => Some(SpecialKey::Tab),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use anyhow::Result;

    use super::Engine;
    use crate::config::{AppConfig, ExpansionRule, MatchBehavior};
    use crate::core::expansion::OutputAction;
    use crate::io::events::{KeyEvent, KeyEventKind, SpecialInputKey};
    use crate::io::output::OutputSink;

    #[derive(Default)]
    struct RecordingSink {
        backspaces: Mutex<Vec<usize>>,
        actions: Mutex<Vec<Vec<OutputAction>>>,
    }

    impl OutputSink for RecordingSink {
        fn send_backspaces(&self, count: usize) -> Result<()> {
            self.backspaces.lock().expect("mutex poisoned").push(count);
            Ok(())
        }

        fn send_actions(&self, actions: &[OutputAction]) -> Result<()> {
            self.actions
                .lock()
                .expect("mutex poisoned")
                .push(actions.to_vec());
            Ok(())
        }
    }

    fn press_char(c: char) -> KeyEvent {
        KeyEvent {
            kind: KeyEventKind::Press,
            printable: Some(c),
            special: None,
            is_injected: false,
        }
    }

    fn press_special(key: SpecialInputKey) -> KeyEvent {
        KeyEvent {
            kind: KeyEventKind::Press,
            printable: None,
            special: Some(key),
            is_injected: false,
        }
    }

    fn test_config(match_behavior: MatchBehavior) -> AppConfig {
        AppConfig {
            expansions: vec![ExpansionRule {
                trigger: ";g".to_string(),
                expansion: "hello".to_string(),
            }],
            match_behavior,
            boundary_chars: None,
        }
    }

    #[test]
    fn immediate_mode_expands_trigger_and_emits_actions() {
        let sink = Arc::new(RecordingSink::default());
        let mut engine = Engine::new(test_config(MatchBehavior::Immediate));
        engine.set_output(sink.clone());

        engine.handle_event(press_char(';')).expect("event should work");
        engine.handle_event(press_char('g')).expect("event should work");

        let backspaces = sink.backspaces.lock().expect("mutex poisoned");
        assert_eq!(&*backspaces, &[2]);

        let actions = sink.actions.lock().expect("mutex poisoned");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].len(), 1);
        match &actions[0][0] {
            OutputAction::Text(text) => assert_eq!(text, "hello"),
            _ => panic!("expected text output action"),
        }
    }

    #[test]
    fn immediate_mode_keeps_buffer_through_modifier_keys() {
        let sink = Arc::new(RecordingSink::default());
        let mut engine = Engine::new(AppConfig {
            expansions: vec![ExpansionRule {
                trigger: "tg@".to_string(),
                expansion: "tylergetsay@gmail.com".to_string(),
            }],
            match_behavior: MatchBehavior::Immediate,
            boundary_chars: None,
        });
        engine.set_output(sink.clone());

        engine.handle_event(press_char('t')).expect("event should work");
        engine.handle_event(press_char('g')).expect("event should work");
        engine
            .handle_event(press_special(SpecialInputKey::Shift))
            .expect("event should work");
        engine.handle_event(press_char('@')).expect("event should work");

        let backspaces = sink.backspaces.lock().expect("mutex poisoned");
        assert_eq!(&*backspaces, &[3]);

        let actions = sink.actions.lock().expect("mutex poisoned");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].len(), 1);
        match &actions[0][0] {
            OutputAction::Text(text) => assert_eq!(text, "tylergetsay@gmail.com"),
            _ => panic!("expected text output action"),
        }
    }
}
