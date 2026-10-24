//! The RON text editor (M5.9, `tasks/plans/m5-editor-v1.md` row 9): every other pack file as
//! text, checked by the loader's own functions before it is written exactly as typed, so the
//! loader and the writer stay one code path and comments survive (ARCHITECTURE §8.2). The model
//! here: which mode the editor is in, the file open as text, and what stops its save. Bevy-free.

/// What the editor's centre shows: the plan, or a file's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// The map's plan, painted on (M5.4 to M5.8).
    #[default]
    Plan,
    /// A pack file as text (M5.9).
    Files,
}

/// A pack file open as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEdit {
    /// Its path under the pack's root (`text/en/maps.ron`).
    pub rel: String,
    /// The text as typed.
    pub text: String,
    /// The text as the file holds it.
    pub disk: String,
    /// The loader's words on the text as typed; none until Check runs on it (typing clears it).
    pub errors: Option<Vec<String>>,
    /// Just opened or reverted: its field is drawn anew with the text and takes the keyboard.
    pub fresh: bool,
}

impl FileEdit {
    /// A file as read from disk.
    #[must_use]
    pub fn read(rel: String, text: String) -> FileEdit {
        FileEdit {
            rel,
            disk: text.clone(),
            text,
            errors: None,
            fresh: true,
        }
    }

    /// Whether the text differs from the file.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.text != self.disk
    }

    /// The text typed: the verdict no longer holds. Whether anything changed.
    pub fn typed(&mut self, text: String) -> bool {
        if self.text == text {
            return false;
        }
        self.text = text;
        self.errors = None;
        true
    }

    /// Check ran: the loader's words on the text as it stands.
    pub fn checked(&mut self, errors: Vec<String>) {
        self.errors = Some(errors);
    }

    /// Why the text cannot be written now, in one line; none when it can: it differs from the
    /// file, Check has run on it, and the loader takes it.
    #[must_use]
    pub fn blocked_save(&self) -> Option<String> {
        if !self.dirty() {
            return Some("no changes".to_owned());
        }
        match &self.errors {
            None => Some("check first".to_owned()),
            Some(errors) => errors.first().map(|first| {
                format!(
                    "{} error{}: {first}",
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" }
                )
            }),
        }
    }

    /// The verdict in a line: not checked, clean, or how many errors.
    #[must_use]
    pub fn verdict(&self) -> String {
        match &self.errors {
            None => "not checked since the last change".to_owned(),
            Some(e) if e.is_empty() => "the loader takes this file".to_owned(),
            Some(e) => format!("{} error{}", e.len(), if e.len() == 1 { "" } else { "s" }),
        }
    }

    /// Back to the file's text, fresh in its field; the verdict goes with the change.
    pub fn revert(&mut self) {
        self.text.clone_from(&self.disk);
        self.errors = None;
        self.fresh = true;
    }

    /// Written: the file holds the text now.
    pub fn written(&mut self) {
        self.disk.clone_from(&self.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_saved_only_changed_and_checked_clean() {
        let mut f = FileEdit::read("text/en/x.ron".into(), "(a: 1)".into());
        assert_eq!(f.blocked_save().as_deref(), Some("no changes"));
        assert!(f.typed("(a: 2)".into()));
        assert!(!f.typed("(a: 2)".into()), "the same text is no change");
        assert_eq!(f.blocked_save().as_deref(), Some("check first"));
        f.checked(vec!["bad".into(), "worse".into()]);
        assert_eq!(f.blocked_save().as_deref(), Some("2 errors: bad"));
        assert_eq!(f.verdict(), "2 errors");
        f.checked(Vec::new());
        assert_eq!(f.blocked_save(), None);
        assert_eq!(f.verdict(), "the loader takes this file");
        f.written();
        assert!(!f.dirty(), "the file holds the text now");
        f.typed("(a: 3)".into());
        f.fresh = false;
        f.revert();
        assert_eq!(f.text, "(a: 2)", "back to what the file holds");
        assert!(f.fresh, "drawn anew with the file's text");
        assert_eq!(f.verdict(), "not checked since the last change");
    }
}
