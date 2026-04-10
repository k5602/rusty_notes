use super::text::Text;

#[derive(Clone, Debug, PartialEq)]
pub struct Search {
    pub text: Text,
}

impl Search {
    pub fn new() -> Search {
        Search { text: Text::new() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Replace {
    pub text: Text,
    stage: ReplaceStage,
}

#[derive(Clone, Debug, PartialEq)]
enum ReplaceStage {
    Search,
    Replacement,
}

impl Replace {
    pub fn new() -> Replace {
        Replace {
            text: Text::from_string("/".to_string()),
            stage: ReplaceStage::Search,
        }
    }

    pub fn search_text(&self) -> String {
        self.text
            .lines
            .first()
            .and_then(|s| s.split('/').next())
            .unwrap_or("")
            .to_string()
    }

    pub fn replacement_text(&self) -> String {
        self.text
            .lines
            .first()
            .and_then(|s| s.split('/').nth(1))
            .unwrap_or("")
            .to_string()
    }

    pub fn get_replacement(&self) -> Option<Text> {
        let line = self.text.lines.first()?;
        let parts: Vec<&str> = line.splitn(3, '/').collect();
        if parts.len() < 2 {
            return None;
        }
        let search = parts[1];
        let replace = parts.get(2).copied().unwrap_or("");
        if search.is_empty() {
            return None;
        }
        let full_text = self.text.lines.join("\n");
        let replaced = full_text.replace(search, replace);
        Some(Text::from_string(replaced))
    }
}
