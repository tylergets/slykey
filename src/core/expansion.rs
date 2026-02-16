use anyhow::{bail, Result};

use crate::io::output::SpecialKey;

#[derive(Debug, Clone)]
pub enum OutputAction {
    Text(String),
    Key(SpecialKey),
    SleepMs(u64),
}

pub fn parse_expansion_actions(input: &str) -> Result<Vec<OutputAction>> {
    let mut actions = Vec::new();
    let mut text_buf = String::new();
    let mut i = 0usize;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        if starts_with_at(bytes, i, b"{{") {
            if !text_buf.is_empty() {
                actions.push(OutputAction::Text(std::mem::take(&mut text_buf)));
            }

            let end = find_macro_end(input, i + 2)
                .ok_or_else(|| anyhow::anyhow!("unclosed macro starting at byte {}", i))?;
            let body = &input[i + 2..end];
            actions.push(parse_macro(body.trim())?);
            i = end + 2;
            continue;
        }

        text_buf.push(input[i..].chars().next().expect("char exists"));
        i += input[i..].chars().next().expect("char exists").len_utf8();
    }

    if !text_buf.is_empty() {
        actions.push(OutputAction::Text(text_buf));
    }

    Ok(actions)
}

fn starts_with_at(haystack: &[u8], index: usize, needle: &[u8]) -> bool {
    haystack.get(index..index + needle.len()) == Some(needle)
}

fn find_macro_end(input: &str, start: usize) -> Option<usize> {
    input[start..].find("}}").map(|offset| start + offset)
}

fn parse_macro(body: &str) -> Result<OutputAction> {
    let (name, value) = body
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("invalid macro format: '{body}'"))?;
    let name = name.trim().to_ascii_uppercase();
    let value = value.trim();

    match name.as_str() {
        "KEY" => Ok(OutputAction::Key(parse_special_key(value)?)),
        "SLEEP_MS" => {
            let ms: u64 = value.parse()?;
            Ok(OutputAction::SleepMs(ms))
        }
        _ => bail!("unsupported macro: '{name}'"),
    }
}

fn parse_special_key(name: &str) -> Result<SpecialKey> {
    let key = match name.to_ascii_uppercase().as_str() {
        "ENTER" | "RETURN" => SpecialKey::Enter,
        "TAB" => SpecialKey::Tab,
        "ESC" | "ESCAPE" => SpecialKey::Escape,
        "BACKSPACE" => SpecialKey::Backspace,
        "SPACE" => SpecialKey::Space,
        "LEFT" => SpecialKey::Left,
        "RIGHT" => SpecialKey::Right,
        "UP" => SpecialKey::Up,
        "DOWN" => SpecialKey::Down,
        "HOME" => SpecialKey::Home,
        "END" => SpecialKey::End,
        "DELETE" => SpecialKey::Delete,
        "PAGEUP" => SpecialKey::PageUp,
        "PAGEDOWN" => SpecialKey::PageDown,
        "F1" => SpecialKey::F1,
        "F2" => SpecialKey::F2,
        "F3" => SpecialKey::F3,
        "F4" => SpecialKey::F4,
        "F5" => SpecialKey::F5,
        "F6" => SpecialKey::F6,
        "F7" => SpecialKey::F7,
        "F8" => SpecialKey::F8,
        "F9" => SpecialKey::F9,
        "F10" => SpecialKey::F10,
        "F11" => SpecialKey::F11,
        "F12" => SpecialKey::F12,
        other => bail!("unknown special key in macro: {other}"),
    };
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::{parse_expansion_actions, OutputAction};
    use crate::io::output::SpecialKey;

    #[test]
    fn parses_plain_text_as_single_action() {
        let actions = parse_expansion_actions("hello world").expect("parsing should succeed");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            OutputAction::Text(text) => assert_eq!(text, "hello world"),
            _ => panic!("expected text action"),
        }
    }

    #[test]
    fn parses_mixed_text_and_macros() {
        let actions = parse_expansion_actions("Hi{{KEY:ENTER}}{{SLEEP_MS:50}}there")
            .expect("parsing should succeed");

        assert_eq!(actions.len(), 4);
        match &actions[0] {
            OutputAction::Text(text) => assert_eq!(text, "Hi"),
            _ => panic!("expected first text action"),
        }
        match actions[1] {
            OutputAction::Key(SpecialKey::Enter) => {}
            _ => panic!("expected enter key action"),
        }
        match actions[2] {
            OutputAction::SleepMs(50) => {}
            _ => panic!("expected sleep action"),
        }
        match &actions[3] {
            OutputAction::Text(text) => assert_eq!(text, "there"),
            _ => panic!("expected trailing text action"),
        }
    }

    #[test]
    fn rejects_unclosed_macro() {
        let err = parse_expansion_actions("x{{KEY:ENTER")
            .expect_err("unclosed macro should return error");
        assert!(err.to_string().contains("unclosed macro"));
    }
}
