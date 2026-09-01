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

    /// Properties is a contextual panel, and the context table lives in the
    /// engine. This reads the panel and holds the two in agreement.
    ///
    /// Three ways this can rot, all silent in a running application: a section
    /// declared and never drawn is a subject with a hole in its editor; a
    /// section drawn but not declared has no subjects, so `sectionApplies`
    /// answers false and it is never seen; and a section drawn under a title
    /// the descriptor does not use gives the same control two names depending
    /// on whether the group is collapsed.
    #[test]
    fn properties_panel_draws_the_sections_the_engine_declares() {
        let text = std::fs::read_to_string(qml_dir().join("PropertiesPanel.qml"))
            .expect("PropertiesPanel.qml is readable from the ui crate");
        let drawn: Vec<(String, String)> = text
            .split("groupId: \"")
            .skip(1)
            .map(|chunk| {
                let id = chunk.split('"').next().unwrap_or_default().to_owned();
                // The title is the next `title: qsTr("…")` after the id; the
                // panel writes the two on consecutive lines.
                let title = chunk
                    .split("title: qsTr(\"")
                    .nth(1)
                    .and_then(|t| t.split('"').next())
                    .unwrap_or_default()
                    .to_owned();
                (id, title)
            })
            .collect();
        assert!(
            drawn.len() > 8,
            "found only {} sections — the scan broke, not the panel",
            drawn.len()
        );

        let declared = phototux_engine::default_disclosure_groups();
        let declared_ids: Vec<&str> = declared.iter().map(|g| g.id.as_str()).collect();
        let drawn_ids: Vec<&str> = drawn.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            drawn_ids, declared_ids,
            "the panel's sections must be the engine's, in the same order"
        );
        for (group, (_, title)) in declared.iter().zip(&drawn) {
            assert_eq!(
                title, &group.title,
                "{} is titled {title:?} in the panel and {:?} in the descriptor",
                group.id, group.title
            );
        }
    }

    /// The panel pins one subject id by hand — the document scope, which is
    /// something it offers rather than something the selection reports. A
    /// typo there would leave the Document tab showing an empty panel.
    #[test]
    fn the_properties_panel_pins_a_subject_the_engine_declares() {
        let text = std::fs::read_to_string(qml_dir().join("PropertiesPanel.qml"))
            .expect("PropertiesPanel.qml is readable from the ui crate");
        let pinned = text
            .split("readonly property string documentSubject: \"")
            .nth(1)
            .and_then(|chunk| chunk.split('"').next())
            .expect("PropertiesPanel declares documentSubject");
        assert_eq!(
            phototux_engine::InspectorSubject::parse(pinned),
            Some(phototux_engine::InspectorSubject::Document),
            "the panel pins subject {pinned:?}, which is not the document"
        );
    }

    /// Lines where `type {` opens a declaration of exactly that type.
    ///
    /// Not [`blocks_of`], which only matches a line that is *nothing but* the
    /// opener. A control is just as unstyled when it is written inline —
    /// `delegate: MenuItem {` is how the layer context menu kept its light
    /// popup through a sweep that converted every other menu in the shell.
    ///
    /// The preceding character has to be a separator, or `ThemedButton {` and
    /// `component LockButton: ...` would both count as bare buttons.
    fn instantiations_of(text: &str, type_name: &str) -> Vec<usize> {
        let opener = format!("{type_name} {{");
        let mut out = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let mut from = 0;
            while let Some(at) = line[from..].find(&opener) {
                let start = from + at;
                let preceded_by_name = start > 0
                    && line[..start]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_');
                if !preceded_by_name {
                    out.push(i + 1);
                    break;
                }
                from = start + opener.len();
            }
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
            // The themed component's own root is a `Slider` with no name; the
            // instances of it are what a screen reader meets.
            if name == "ThemedSlider.qml" {
                continue;
            }
            for (line, body) in blocks_of(&text, "ThemedSlider") {
                checked += 1;
                assert!(
                    body.contains("Accessible.name"),
                    "{name}:{line} is a slider with no Accessible.name — a screen \
                     reader announces it as an unnamed slider"
                );
            }
        }
        // A floor, not a count: it catches the scan silently matching nothing.
        // It came down from 15 when Properties stopped carrying a second copy
        // of the brush sliders and the foreground channels — those live in the
        // options bar and the Swatches panel — so the shell has fewer sliders
        // than it did, not fewer named ones.
        assert!(
            checked > 10,
            "found {checked} sliders — the scan broke rather than the shell"
        );
    }

    /// Bare `Button` reaches the user as a white rectangle.
    ///
    /// No Controls style is configured, so the shell runs the **Basic** style,
    /// whose palette is hardcoded light and ignores `palette`. That is
    /// invisible on a developer profile with a Qt style set system-wide and
    /// obvious on a clean one — which is every user's profile — so it survived
    /// in thirty-seven places while four call sites hand-wrote the same
    /// rounded rectangle to work around it. `ThemedButton` is the one home for
    /// that treatment; the same rule already applies to check boxes, combo
    /// boxes and spin boxes.
    #[test]
    fn no_unstyled_controls_reach_the_user() {
        for (name, text) in qml_files() {
            for (bare, themed) in [
                ("Button", "ThemedButton"),
                ("CheckBox", "ThemedCheckBox"),
                ("ComboBox", "ThemedComboBox"),
                ("SpinBox", "ThemedSpinBox"),
                ("Slider", "ThemedSlider"),
                ("TextField", "ThemedTextField"),
                ("Menu", "ThemedMenu"),
                ("MenuItem", "ThemedMenuItem"),
            ] {
                // The themed component is allowed to *be* the bare control.
                // `ThemedMenu` also names `ThemedMenuItem` as its delegate,
                // which is the whole point of it.
                if name == format!("{themed}.qml") || name.starts_with("Themed") {
                    continue;
                }
                let found = instantiations_of(&text, bare);
                assert!(
                    found.is_empty(),
                    "{name} instantiates a bare {bare} at {found:?}, which the Basic \
                     style draws with a hardcoded light palette — use {themed}"
                );
            }
        }
    }

    /// The status bar carries state, never messages.
    ///
    /// It shows the document summary — size, zoom, active layer, tool — which
    /// is true continuously. A message is true once. When both went into the
    /// same string the next summary refresh silently erased whatever the user
    /// had not yet read, and there are six places that refresh it. Messages go
    /// to the toast channel; `status_text` may only ever be the summary.
    #[test]
    fn nothing_writes_a_message_into_the_status_bar() {
        let source = include_str!("lib.rs");
        let mut assignments = 0;
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            let Some(rhs) = trimmed.strip_prefix("self.status_text = ") else {
                continue;
            };
            assignments += 1;
            assert_eq!(
                rhs,
                "self.engine.status_summary();",
                "lib.rs:{} assigns something other than the summary to the status \
                 bar — messages belong in `notify`, which the next summary \
                 refresh cannot erase",
                i + 1
            );
        }
        assert!(
            assignments > 3,
            "found {assignments} status_text assignments — the scan broke rather \
             than the shell"
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
