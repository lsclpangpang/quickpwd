//! 快捷键字符串（accelerator，如 "Control+Shift+KeyK"）与虚拟键码之间的转换。
//! 按键名与浏览器 KeyboardEvent.code 及 global-hotkey 的解析格式一致。

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

impl Mods {
    pub fn any(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.win
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Combo {
    pub mods: Mods,
    pub vk: u32,
}

const NAMED_KEYS: &[(&str, u32)] = &[
    ("Space", 0x20),
    ("Enter", 0x0D),
    ("Tab", 0x09),
    ("Backspace", 0x08),
    ("Escape", 0x1B),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("Home", 0x24),
    ("End", 0x23),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("ArrowLeft", 0x25),
    ("ArrowUp", 0x26),
    ("ArrowRight", 0x27),
    ("ArrowDown", 0x28),
    ("Backquote", 0xC0),
    ("Minus", 0xBD),
    ("Equal", 0xBB),
    ("BracketLeft", 0xDB),
    ("BracketRight", 0xDD),
    ("Backslash", 0xDC),
    ("Semicolon", 0xBA),
    ("Quote", 0xDE),
    ("Comma", 0xBC),
    ("Period", 0xBE),
    ("Slash", 0xBF),
];

pub fn vk_to_code(vk: u32) -> Option<String> {
    match vk {
        0x41..=0x5A => Some(format!("Key{}", char::from_u32(vk)?)),
        0x30..=0x39 => Some(format!("Digit{}", vk - 0x30)),
        0x60..=0x69 => Some(format!("Numpad{}", vk - 0x60)),
        0x70..=0x87 => Some(format!("F{}", vk - 0x6F)),
        _ => NAMED_KEYS.iter().find(|(_, v)| *v == vk).map(|(n, _)| n.to_string()),
    }
}

fn code_to_vk(code: &str) -> Option<u32> {
    if let Some(c) = code.strip_prefix("Key") {
        let ch = c.chars().next()?;
        return (c.len() == 1 && ch.is_ascii_uppercase()).then_some(ch as u32);
    }
    if let Some(d) = code.strip_prefix("Digit") {
        return d.parse::<u32>().ok().filter(|n| *n <= 9).map(|n| 0x30 + n);
    }
    if let Some(d) = code.strip_prefix("Numpad") {
        return d.parse::<u32>().ok().filter(|n| *n <= 9).map(|n| 0x60 + n);
    }
    if let Some(n) = code.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        return (1..=24).contains(&n).then_some(0x6F + n);
    }
    NAMED_KEYS.iter().find(|(n, _)| *n == code).map(|(_, v)| *v)
}

pub fn parse(acc: &str) -> Option<Combo> {
    let mut mods = Mods::default();
    let mut vk = None;
    for token in acc.split('+') {
        match token {
            "Control" => mods.ctrl = true,
            "Alt" => mods.alt = true,
            "Shift" => mods.shift = true,
            "Super" => mods.win = true,
            key if vk.is_none() => vk = Some(code_to_vk(key)?),
            _ => return None,
        }
    }
    Some(Combo { mods, vk: vk? })
}

pub fn format(mods: Mods, code: Option<&str>) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if mods.ctrl {
        parts.push("Control");
    }
    if mods.alt {
        parts.push("Alt");
    }
    if mods.shift {
        parts.push("Shift");
    }
    if mods.win {
        parts.push("Super");
    }
    if let Some(c) = code {
        parts.push(c);
    }
    parts.join("+")
}

/// "Control+Shift+KeyK" -> "Ctrl + Shift + K"
pub fn label(acc: &str) -> String {
    acc.split('+')
        .map(|t| match t {
            "Control" => "Ctrl".to_string(),
            "Super" => "Win".to_string(),
            "Backquote" => "`".to_string(),
            t if t.starts_with("Key") && t.len() == 4 => t[3..].to_string(),
            t if t.starts_with("Digit") && t.len() == 6 => t[5..].to_string(),
            t => t.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// 这些组合由系统处理（Win 组合）或会触发 Alt 菜单，必须用键盘钩子接管
pub fn needs_hook(c: &Combo) -> bool {
    c.mods.win || (c.mods.alt && !c.mods.ctrl && !c.mods.shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for acc in ["Alt+Space", "Control+Shift+KeyK", "Super+Space", "Control+Backquote", "F5", "Alt+Digit1"] {
            let c = parse(acc).unwrap();
            assert_eq!(format(c.mods, vk_to_code(c.vk).as_deref()), acc);
        }
        assert_eq!(label("Control+Shift+KeyK"), "Ctrl + Shift + K");
        assert!(parse("Control+Bogus").is_none());
        assert!(needs_hook(&parse("Super+Space").unwrap()));
        assert!(needs_hook(&parse("Alt+Space").unwrap()));
        assert!(!needs_hook(&parse("Control+Alt+Space").unwrap()));
    }
}
