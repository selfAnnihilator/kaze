use std::collections::HashSet;

/// A selection starts the profile. Qualified listening can add new languages.
#[derive(Debug, Clone, Default)]
pub(crate) struct LanguagePreference {
    selected: HashSet<String>,
    heard: HashSet<String>,
}

impl LanguagePreference {
    pub(super) fn new(selected: &[String], heard: HashSet<String>) -> Self {
        Self {
            selected: selected.iter().map(|name| language_key(name)).collect(),
            heard,
        }
    }

    pub(crate) fn allows(&self, language: Option<&str>) -> bool {
        self.selected.is_empty()
            || language.is_some_and(|name| {
                let key = language_key(name);
                self.selected.contains(&key) || self.heard.contains(&key)
            })
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.selected.is_empty()
    }

    pub(crate) fn seed_languages(&self, selection: &[String]) -> Vec<String> {
        if selection.is_empty() {
            return vec![];
        }
        let mut languages: Vec<_> = selection.iter().map(|name| language_key(name)).collect();
        let mut heard: Vec<_> = self.heard.iter().cloned().collect();
        heard.sort();
        for language in heard {
            if !languages.contains(&language) {
                languages.push(language);
            }
        }
        languages
    }
}

pub(crate) fn language_key(name: &str) -> String {
    name.trim().to_lowercase()
}
