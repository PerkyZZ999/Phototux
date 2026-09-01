//! Contracts the QML shell must keep, checked from Rust.
//!
//! `qml/` has no test runner of its own, and the properties here are the kind
//! that fail silently in a running application: a slider that a screen reader
//! cannot name, a delegate role that renders nothing. They are checked by
//! reading the shell as text — the same approach the engine already uses for
//! icon packaging and menu structure — because the alternative is a second list
//! that someone has to remember to update.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn qml_dir() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml"))
    }

    /// Every `.qml` file in the shell, as `(name, text)`.
    fn qml_files() -> Vec<(String, String)> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(qml_dir())
            .expect("qml/ is readable from the ui crate")
            .flatten()
        {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "qml") {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push((
                name,
                std::fs::read_to_string(&path).expect("qml is readable"),
            ));
        }
        assert!(
            !out.is_empty(),
            "no QML found — the scan broke, not the shell"
        );
        out
    }

    /// The `type {` blocks in `text`, as `(line number, body)`.
    ///
    /// Brace counting rather than a parser: the shell is hand-written QML with
    /// one declaration per line, and a real parse would be a dependency and a
    /// second thing to get right for a check this simple.
    fn blocks_of(text: &str, type_name: &str) -> Vec<(usize, String)> {
        let lines: Vec<&str> = text.lines().collect();
        let opener = format!("{type_name} {{");
        let mut out = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line.trim() != opener {
                continue;
            }
            let mut depth = 0_i32;
            let mut body = String::new();
            for line in &lines[i..] {
                depth += line.matches('{').count() as i32;
                depth -= line.matches('}').count() as i32;
                body.push_str(line);
                body.push('\n');
                if depth <= 0 {
                    break;
                }
            }
            out.push((i + 1, body));
        }
        out
    }

    /// A slider has no text of its own, so without a name it reaches assistive
    /// technology as an anonymous "slider".
    ///
    /// Buttons and checkboxes are excluded on purpose: they carry visible text,
    /// which Qt uses as the accessible name already. A slider's label sits in a
    /// separate `Label` beside it, and nothing connects the two.
    #[test]
    fn every_slider_tells_assistive_technology_what_it_adjusts() {
        let mut checked = 0;
        for (name, text) in qml_files() {
            for (line, body) in blocks_of(&text, "Slider") {
                checked += 1;
                assert!(
                    body.contains("Accessible.name"),
                    "{name}:{line} is a Slider with no Accessible.name — a screen \
                     reader announces it as an unnamed slider"
                );
            }
        }
        assert!(
            checked > 15,
            "found {checked} sliders — the scan broke rather than the shell"
        );
    }

    /// Icon-only buttons have no text to fall back on either.
    #[test]
    fn every_icon_only_tool_button_is_named() {
        let mut checked = 0;
        for (name, text) in qml_files() {
            for (line, body) in blocks_of(&text, "ChromeIconToolButton") {
                checked += 1;
                assert!(
                    body.contains("Accessible.name") || body.contains("text:"),
                    "{name}:{line} is an icon-only button with no Accessible.name"
                );
            }
        }
        assert!(checked > 3, "found {checked} icon buttons — the scan broke");
    }
}
