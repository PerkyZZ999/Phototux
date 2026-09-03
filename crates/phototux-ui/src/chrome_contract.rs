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

    /// The braced body of the block opening on 1-based `line`.
    ///
    /// The scans above ask whether a control overrides something, and
    /// "somewhere later in the file" is not an answer — a `background:` two
    /// controls down would satisfy a plain `contains`. Matching braces keeps
    /// the question about the control that was found.
    fn body_at_line(text: &str, line: usize) -> String {
        let offset: usize = text
            .lines()
            .take(line.saturating_sub(1))
            .map(|l| l.len() + 1)
            .sum();
        let Some(open) = text[offset..].find('{').map(|i| offset + i) else {
            return String::new();
        };
        let mut depth = 0usize;
        for (i, ch) in text[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return text[open..open + i].to_owned();
                    }
                }
                _ => {}
            }
        }
        text[open..].to_owned()
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
                ("DialogButtonBox", "ThemedDialogFooter"),
                ("ScrollBar", "ThemedScrollBar"),
                ("ToolTip", "ThemedToolTip"),
                // Basic draws this one with `pen: palette.dark`, so the
                // status bar's "Working…" spinner was a dark grey smudge on
                // dark chrome — and Basic's own `padding: 6` around a
                // 48-pixel contentItem left about six pixels of it inside the
                // eighteen the status bar allows.
                ("BusyIndicator", "ThemedBusyIndicator"),
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

    /// A text field that displays a document value binds `source`, not `text`.
    ///
    /// Qt drops a `TextField`'s `text` binding the moment the user types into
    /// it, and nothing puts it back. Every field here that shows a document
    /// value was one rejected keystroke from showing something the document
    /// does not have: typing `notacolour` into the swatches hex and pressing
    /// Return left `notacolour` on screen for the rest of the session while the
    /// swatch beside it never moved. Undo is the other way in — Ctrl+Z inside a
    /// focused field is the field's own undo.
    ///
    /// `ThemedTextField.source` is a plain property, so it cannot lose its
    /// binding, and its change handler writes `text`. `Qt.binding` looked like
    /// the fix and was not reliable: with a conditional source the field kept
    /// showing the wrong half of a pair through a sequence of perfectly
    /// ordinary clicks.
    #[test]
    fn a_field_that_shows_a_value_binds_source_not_text() {
        let mut checked = 0;
        for (name, text) in qml_files() {
            if name == "ThemedTextField.qml" {
                continue;
            }
            for start in instantiations_of(&text, "ThemedTextField") {
                let body = body_at_line(&text, start);
                let commits = body.contains("onEditingFinished")
                    || body.contains("Keys.onReturnPressed")
                    || body.contains("onAccepted");
                if !commits {
                    continue;
                }
                checked += 1;
                let binds_text = body
                    .lines()
                    .any(|l| l.trim_start().starts_with("text:") && !l.contains("text: \""));
                assert!(
                    !binds_text,
                    "{name}:{start} binds `text` to a value and commits edits — \
                     the binding is lost on the first keystroke and the field \
                     then shows whatever was typed. Bind `source` instead."
                );
            }
        }
        assert!(
            checked >= 5,
            "found {checked} editable fields that commit — the scan broke rather \
             than the shell"
        );
    }

    /// A kind-gated enablement compares against a string, and the string has
    /// to be one a layer can actually report.
    ///
    /// `action_enablement` decides `text_layer`, `shape_layer`,
    /// `group_selected` and `smart_object` by comparing `active_layer_kind`
    /// with a literal. That literal comes from `LayerKind::as_str`, which this
    /// crate does not consult — so renaming a kind in the engine, or writing
    /// `"smart object"` for `"smart-object"`, leaves the comparison silently
    /// false and the menu entry permanently greyed out. Nothing else fails
    /// first: a menu item that is never enabled looks exactly like a menu item
    /// that is correctly disabled.
    #[test]
    fn every_kind_an_enablement_names_is_a_kind_a_layer_reports() {
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
            .expect("the host crate is readable from its own tests");
        let kinds: Vec<&str> = phototux_engine::LayerKind::ALL
            .iter()
            .map(|k| k.as_str())
            .collect();
        let mut checked = 0;
        for (i, _) in source.match_indices("active_layer_kind == \"") {
            let rest = &source[i + "active_layer_kind == \"".len()..];
            let named = rest.split('"').next().expect("a closing quote");
            assert!(
                kinds.contains(&named),
                "an enablement compares active_layer_kind with {named:?}, which no \
                 LayerKind reports — the entry it gates can never be enabled. \
                 Kinds are {kinds:?}"
            );
            checked += 1;
        }
        assert!(
            checked >= 3,
            "found {checked} kind comparisons — the scan broke rather than the host"
        );
    }

    /// A dialog laid out in raw pixels crowds its own content at the other
    /// density.
    ///
    /// `Theme.densityScale` scales every type and spacing token, so a dialog
    /// pinned to `width: 720` keeps its box while the words inside it grow. At
    /// Comfortable, New Document ran "Recommended" and "1920 x 1080" into the
    /// preset card's border — the four cards share the row, so each one shrank
    /// exactly as its text got bigger.
    ///
    /// The check is on the root `width:` of a dialog file, which is the one
    /// that sets the box. Inner widths are the layout's business.
    #[test]
    fn no_dialog_pins_itself_to_a_pixel_width() {
        let mut checked = 0;
        for (name, text) in qml_files() {
            if !name.ends_with("Dialog.qml") {
                continue;
            }
            for (i, line) in text.lines().enumerate() {
                // Root-level property: exactly four spaces of indent.
                let Some(rest) = line.strip_prefix("    width:") else {
                    continue;
                };
                if !line.starts_with("    width:") || line.starts_with("     ") {
                    continue;
                }
                checked += 1;
                assert!(
                    rest.contains("densityScale") || rest.contains("parent") || rest.contains('%'),
                    "{name}:{} pins the dialog to a pixel width, so its content \
                     crowds at the other UI density — scale it by \
                     Theme.densityScale",
                    i + 1
                );
            }
        }
        assert!(
            checked >= 6,
            "found {checked} dialog widths — the scan broke rather than the shell"
        );
    }

    /// A combo box's label is a `Label` beside it, and nothing connects them.
    ///
    /// Same shape as the slider check below: the control carries no text of its
    /// own, so without a name it reaches assistive technology as an anonymous
    /// "combo box" and the user has to guess from the selected value what it
    /// selects. Four of them shipped that way — the effect picker, UI density,
    /// font family and text alignment.
    ///
    /// A component *definition* and a `delegate:` are exempt: their instances
    /// carry the text, which Qt uses as the name already.
    #[test]
    fn every_combo_box_tells_assistive_technology_what_it_selects() {
        let mut checked = 0;
        for (name, text) in qml_files() {
            if name == "ThemedComboBox.qml" {
                continue;
            }
            for start in instantiations_of(&text, "ThemedComboBox") {
                let opener = text
                    .lines()
                    .nth(start.saturating_sub(1))
                    .unwrap_or_default();
                if opener.contains("component ") || opener.contains("delegate:") {
                    continue;
                }
                let body = body_at_line(&text, start);
                checked += 1;
                assert!(
                    body.contains("Accessible.name"),
                    "{name}:{start} is an unnamed combo box — its label is a \
                     Label beside it, which nothing connects to it"
                );
            }
        }
        assert!(
            checked >= 4,
            "found {checked} combo boxes — the scan broke rather than the shell"
        );
    }

    /// A `ToolButton` that keeps the Basic background is a pale grey square.
    ///
    /// Not on the unstyled-control list above, because a `ToolButton` with its
    /// own `contentItem` *and* `background` is the right thing to write —
    /// `ChromeIconToolButton` is exactly that. What is never right is leaving
    /// the background: this Qt's Basic style paints `palette.button` at 0.5
    /// opacity with a `palette.windowText` border, and unlike older versions
    /// there is no `visible:` guard limiting it to the pressed state. Two
    /// effect-reorder buttons in the Properties panel sat there as permanent
    /// pale rectangles on dark chrome.
    #[test]
    fn every_tool_button_replaces_the_basic_background() {
        let mut checked = 0;
        for (name, text) in qml_files() {
            for start in instantiations_of(&text, "ToolButton") {
                let body = body_at_line(&text, start);
                checked += 1;
                assert!(
                    body.contains("background:"),
                    "{name}:{start} leaves a ToolButton's Basic background, which \
                     paints a pale rectangle on dark chrome — give it one, or use \
                     ChromeIconToolButton"
                );
            }
        }
        assert!(
            checked > 8,
            "found {checked} tool buttons — the scan broke rather than the shell"
        );
    }

    /// Parking a document must drop the host-side undo stacks with it.
    ///
    /// It already does; this pins it. Selection and transform undo are `Vec`s
    /// on `AppSession`, not fields of the parked `SessionState`, so they do not
    /// travel with the document the way everything else here does — and the
    /// engine's history *is* per document. Drop the clearing and undoing in the
    /// next tab pops that tab's `Selection` entry while the host restores the
    /// previous tab's mask over it, a mask that need not even be the same size.
    ///
    /// Nothing else states the requirement, and the two calls sit twenty lines
    /// below the `mem::take` that makes them necessary, which is exactly the
    /// distance at which a tidy-up removes them. Read from the source because
    /// `AppSession` needs an attached `QObject` and cannot be built in a unit
    /// test.
    #[test]
    fn parking_a_document_drops_the_host_undo_stacks() {
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
            .expect("the host crate is readable from its own tests");
        let at = source
            .find("fn park_current_document(&mut self)")
            .expect("park_current_document is there to read");
        // Newlines, not `lines().count()`: the text before `at` ends mid-line
        // (the indent before `fn`), so `lines()` already counts that line and
        // adding one lands on the line after the signature.
        let line = source[..at].matches('\n').count() + 1;
        let body = body_at_line(&source, line);
        assert!(
            body.contains("park_active("),
            "the slice missed the function body — the scan broke rather than \
             the host"
        );
        for call in ["clear_selection_stacks()", "clear_transform_stacks()"] {
            assert!(
                body.contains(call),
                "park_current_document does not call {call}, so the stacks \
                 survive into the next tab and undo restores another \
                 document's mask"
            );
        }
    }

    /// A long file operation must be stoppable.
    ///
    /// `cancel_io` sets a token the worker checks between layers, and `send`
    /// resets it before every command, so cancelling has always worked and
    /// has never poisoned the next save. Nothing called the slot. Saving a
    /// large PSD or exporting a 4K composite is the one thing here that takes
    /// long enough to regret starting, and the status bar showed a spinner
    /// and the word "Working…" with no way out of it.
    ///
    /// Pinned as a pair: the call and the `ioBusy` guard. A cancel button that
    /// is always on screen offers to stop something that is not running.
    #[test]
    fn a_running_file_operation_can_be_cancelled() {
        let shell = qml_files()
            .into_iter()
            .find(|(name, _)| name == "Main.qml")
            .map(|(_, text)| text)
            .expect("the shell is readable");
        assert!(
            shell.contains("AppSession.cancelIo()"),
            "nothing in the shell calls cancelIo, so a running save cannot be stopped"
        );
        let offered = shell
            .lines()
            .position(|line| line.contains("AppSession.cancelIo()"))
            .expect("the call is there");
        let preceding: Vec<&str> = shell.lines().take(offered).collect();
        let guarded = preceding
            .iter()
            .rev()
            .take(12)
            .any(|line| line.contains("visible: AppSession.ioBusy"));
        assert!(
            guarded,
            "the cancel control is not guarded by ioBusy, so it offers to stop \
             an operation that is not running"
        );
    }

    /// The attached tool tip is the Basic style's, and it is light.
    ///
    /// `ToolTip.visible` / `ToolTip.text` on a control drive the **shared**
    /// tool tip instance, which the Controls style builds — Basic, hardcoded
    /// light palette. That is not a bare `ToolTip {` the check above would
    /// catch: nothing is instantiated, so forty call sites popped pale grey
    /// tips over dark chrome with nothing at the call site to show for it.
    ///
    /// The shared instance cannot be restyled from one place; assigning to
    /// `ToolTip.toolTip.background` is accepted and does nothing, from an Item
    /// or from the window. `ThemedToolTip` is a popup the call site owns.
    #[test]
    fn no_attached_tool_tips_reach_the_user() {
        for (name, text) in qml_files() {
            if name == "ThemedToolTip.qml" {
                continue;
            }
            let found: Vec<usize> = text
                .lines()
                .enumerate()
                .filter(|(_, line)| {
                    let t = line.trim_start();
                    t.starts_with("ToolTip.visible")
                        || t.starts_with("ToolTip.text")
                        || t.starts_with("ToolTip.delay")
                        || t.starts_with("ToolTip.timeout")
                })
                .map(|(i, _)| i + 1)
                .collect();
            assert!(
                found.is_empty(),
                "{name} drives the shared tool tip at {found:?}, which the Basic \
                 style draws with a hardcoded light palette — declare a \
                 ThemedToolTip inside the control instead"
            );
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

    /// The tab strip is refreshed wherever the document fields are.
    ///
    /// A tab's label and dirty dot are `document_name` and `dirty` — the same
    /// two values the window title binds to — but they reach QML through a
    /// pushed JSON string rather than through those properties, so they go
    /// stale wherever a caller re-emits the properties and forgets the strip.
    /// Save As did exactly that: the window title took the new file name and
    /// the tab went on reading "Untitled". `emit_doc_fields` is the one place
    /// every such caller already goes through.
    #[test]
    fn the_tab_strip_is_refreshed_with_the_document_fields() {
        let source = include_str!("lib.rs");
        let start = source
            .find("fn emit_doc_fields(&mut self) {")
            .expect("emit_doc_fields exists");
        let body_end = source[start..]
            .find("\n    }")
            .expect("emit_doc_fields is closed");
        let body = &source[start..start + body_end];
        assert!(
            body.contains("self.refresh_document_tabs_json();"),
            "emit_doc_fields no longer refreshes the tab strip, so every caller \
             that renames or cleans a document leaves the tab reading the old \
             name with the old dirty dot"
        );
    }

    /// The status bar states each fact once.
    ///
    /// `status_summary` is the engine's account of document state — size,
    /// zoom, active layer, edit target, selection, layer count, tool — and it
    /// reaches the bar as `statusText`. Four items to its right sat a second
    /// label reading the zoom off `AppSession.zoom` and printing the same
    /// number again. That cluster is per-frame metrics: composite time, frame
    /// rate, the GPU badge, all of them things kept *out* of the summary
    /// because they would churn its AT-SPI name every frame. Document state
    /// does not belong in it twice.
    #[test]
    fn the_status_bar_does_not_repeat_the_document_summary() {
        let main = qml_files()
            .into_iter()
            .find(|(name, _)| name == "Main.qml")
            .map(|(_, text)| text)
            .expect("Main.qml is readable");
        let (line, row) = blocks_of(&main, "RowLayout")
            .into_iter()
            .find(|(_, body)| body.contains("AppSession.statusText"))
            .expect("the status bar is a RowLayout carrying statusText");
        for field in [
            "AppSession.zoom",
            "AppSession.docWidth",
            "AppSession.docHeight",
            "AppSession.activeTool",
            "AppSession.activeLayerName",
        ] {
            assert!(
                !row.contains(field),
                "the status bar at Main.qml:{line} reads {field}, which \
                 `status_summary` already states — say it once, in the summary"
            );
        }
    }

    /// File dialogs open where the user last was, never where they were built.
    ///
    /// Each `FileDialog` keeps its own `currentFolder`, so calling `open()`
    /// directly reopens whichever folder *that* dialog last saw — Open, Save
    /// As, Export and Embed ICC each remembering a different place, and none
    /// of them the document the user has in front of them. `browseForFile`
    /// resolves the folder for all four; a bare `open()` quietly opts out.
    #[test]
    fn every_file_dialog_opens_where_the_user_last_was() {
        let mut checked = 0;
        for (name, text) in qml_files() {
            let dialogs: Vec<&str> = text
                .lines()
                .filter_map(|line| line.trim().strip_prefix("id: "))
                .filter(|id| id.ends_with("FileDialog"))
                .collect();
            for id in dialogs {
                checked += 1;
                let bare = format!("{id}.open()");
                assert!(
                    !text.contains(&bare),
                    "{name} calls {bare} directly — route it through \
                     root.browseForFile({id}) so it opens in the document's \
                     folder rather than wherever that dialog was last used"
                );
            }
        }
        assert!(
            checked >= 4,
            "found {checked} file dialogs — the scan broke rather than the shell"
        );
    }
}
