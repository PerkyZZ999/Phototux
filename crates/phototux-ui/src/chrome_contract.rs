//! Contracts the QML shell must keep, checked from Rust.
//!
//! `qml/` has no test runner of its own, and the properties here are the kind
//! that fail silently in a running application: a slider that a screen reader
//! cannot name, a delegate role that renders nothing. They are checked by
//! reading the shell as text — the same approach the engine already uses for
//! icon packaging and menu structure — because the alternative is a second list
//! that someone has to remember to update.

/// The shell's source directory.
#[cfg(test)]
pub(crate) fn qml_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml"))
}

/// Every `.qml` file in the shell, as `(name, text)`.
///
/// Shared with the guards in `lib.rs` rather than written out again there.
/// Reading the shell is how most of these contracts are checked at all, so
/// the walk is the first twenty lines of every one — which is enough friction
/// that a guard worth having does not get written. Behind one call it costs
/// three lines, and the parts that are easy to get subtly different each time
/// — skipping non-`.qml`, carrying the file name into the failure message,
/// and failing loudly when the scan itself breaks rather than passing on an
/// empty corpus — are decided once.
#[cfg(test)]
pub(crate) fn qml_files() -> Vec<(String, String)> {
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

#[cfg(test)]
mod tests {
    use super::{qml_dir, qml_files};

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
    /// The fill-layer colour belongs to the fill-layer inspector, nowhere else.
    ///
    /// `fillColorHex` / `setActiveFillHex` describe a `LayerKind::Fill` layer's
    /// colour and route to `layer.set-fill-color`. The Paint Bucket's options
    /// bar borrowed them, which put a colour the tool does not use in front of
    /// the user — `fillActiveLayer` pours `engine.colors.foreground` — and made
    /// editing that field try to recolour a fill layer, which a raster layer
    /// refuses. The field showed `#738CBF`, the inspector's default for a layer
    /// with no fill content, while the bucket poured black.
    ///
    /// Two properties one character apart in meaning is exactly the pair to
    /// pin: a control that shows one value and acts on another looks like it
    /// works.
    #[test]
    fn only_the_fill_layer_inspector_speaks_for_a_fill_layers_colour() {
        for (name, source) in qml_files() {
            if name == "PropertiesPanel.qml" {
                continue;
            }
            // The `AppSession.` prefix is what makes this a binding rather
            // than prose — the comment beside the fix names both properties,
            // and a guard that fails on its own explanation is a nuisance.
            for property in ["AppSession.fillColorHex", "AppSession.setActiveFillHex"] {
                assert!(
                    !source.contains(property),
                    "{name} binds {property}, which describes a fill *layer*. A tool \
                     that pours the foreground must bind foregroundHex / \
                     setForegroundHex, or it shows one colour and uses another"
                );
            }
        }
    }

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
        // Joined onto one line first. `rustfmt` wraps a long assignment after
        // the `=`, and a line-at-a-time scan cannot see the right-hand side of
        // one that did: the colour-profile conversion wrote a destructive-edit
        // warning into the status bar for months in plain sight of this test.
        let source: String = source
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .replace("=\n", "= ");
        let mut assignments = 0;
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            // Any binding, not just `self`. Matching only `self.status_text`
            // missed three writers: `Default` builds the session in locals
            // named `session` and `out`, and all three of those assignments
            // were messages — including an "Opening …" the status bar went on
            // showing after the open had failed.
            let Some((binding, rhs)) = trimmed.split_once(".status_text = ") else {
                continue;
            };
            if !binding
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                || binding.is_empty()
            {
                continue;
            }
            assignments += 1;
            assert_eq!(
                rhs,
                format!("{binding}.engine.status_summary();"),
                "lib.rs:{} assigns something other than the summary to the status \
                 bar — messages belong in `notify`, or in \
                 `queue_notice_before_proxy` when the QObject proxy does not \
                 exist yet, because the next summary refresh erases the status bar",
                i + 1
            );
        }
        assert!(
            assignments > 3,
            "found {assignments} status_text assignments — the scan broke rather \
             than the shell"
        );
    }

    /// A slot that changes the workspace layout must republish it.
    ///
    /// `persist_workspace_visibility` is the only thing that tells the shell:
    /// it applies the workspace to preferences, writes them, and emits the
    /// five properties QML binds the dock to. A slot that mutates
    /// `self.workspace` without reaching it — directly, or through
    /// `commit_workspace_op` — leaves the user looking at the layout they had
    /// before.
    ///
    /// Window ▸ Reset Workspace did exactly that. It reset the workspace and
    /// the stored preferences and emitted the preference fields, which carry
    /// panel *visibility* but not the dock, so an auto-hidden panel stayed
    /// hidden behind a toast reading "Workspace reset to Essentials".
    ///
    /// The exceptions are the parts of `WorkspaceState` that are not layout:
    /// focus, which is neither persisted nor drawn by the dock and publishes
    /// its own JSON, and `active_preset_id`, which is one property with its own
    /// notify. Naming exceptions rather than subjects is deliberate — a new
    /// mutating slot fails this test until someone decides which it is.
    #[test]
    fn a_slot_that_changes_the_workspace_layout_republishes_it() {
        const NOT_LAYOUT: [&str; 3] = ["set_focus_path", "set_panel_context", "active_preset_id"];
        let source = include_str!("lib.rs");
        let mut checked = 0;
        for block in source.split("    #[qslot]").skip(1) {
            // One slot: up to the next impl-level item, with the comments
            // dropped. A doc comment that *names* `persist_workspace_visibility`
            // — such as the one inside `reset_workspace` explaining why it is
            // called — otherwise satisfies the check by itself, and the guard
            // passes over the very defect it was written for.
            let body: String = block
                .split("\n    }")
                .next()
                .unwrap_or(block)
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            let body = body.as_str();
            let name = body
                .split("fn ")
                .nth(1)
                .and_then(|rest| rest.split('(').next())
                .unwrap_or("?")
                .trim();
            let touches: Vec<&str> = body
                .match_indices("self.workspace.")
                .filter_map(|(at, _)| {
                    body[at + "self.workspace.".len()..]
                        .split(['(', ' ', '.', ';', ')'])
                        .next()
                })
                .collect();
            if touches.is_empty() || touches.iter().all(|m| NOT_LAYOUT.contains(m)) {
                continue;
            }
            checked += 1;
            assert!(
                body.contains("persist_workspace_visibility")
                    || body.contains("commit_workspace_op"),
                "`{name}` changes the workspace ({}) without republishing it — \
                 call `persist_workspace_visibility`, or `commit_workspace_op` \
                 when the change can be refused",
                touches.join(", ")
            );
        }
        assert!(
            checked > 3,
            "found {checked} workspace-mutating slots — the scan broke rather \
             than the shell"
        );
    }

    /// Pixels written to the GPU have to reach the screen.
    ///
    /// The host writes layer and mask textures directly through
    /// `phototux_canvas`, which changes what the canvas *would* draw but does
    /// not ask it to draw anything. `recomposite` is that request. A handler
    /// that writes pixels and does not make it leaves the user looking at the
    /// composite from before the edit, with no error and nothing to click:
    /// `Select ▸ Selection to Mask` wrote the mask and looked like a no-op
    /// until some unrelated edit forced a repaint.
    ///
    /// Three writers legitimately do not call it, and are named here rather
    /// than inferred: the two load paths present through `record_composite`
    /// once the whole document is in place, and the colour-profile conversion
    /// is a follow-up whose `CommandEffects` carries `recomposite` for it.
    #[test]
    fn a_handler_that_writes_pixels_asks_for_a_new_frame() {
        const PRESENTS_ANOTHER_WAY: [&str; 3] = [
            "finish_opened_ptx",
            "open_psd_pixels",
            "apply_convert_pixels",
        ];
        let source = include_str!("lib.rs");
        let mut checked = 0;
        let mut current = "?";
        let mut body = String::new();
        let mut bodies: Vec<(&str, String)> = Vec::new();
        for line in source.lines() {
            let is_item = line.starts_with("    fn ")
                || line.starts_with("    pub fn ")
                || line.starts_with("    pub(crate) fn ");
            if is_item {
                bodies.push((current, std::mem::take(&mut body)));
                current = line
                    .split("fn ")
                    .nth(1)
                    .and_then(|rest| rest.split('(').next())
                    .unwrap_or("?")
                    .trim();
            }
            if !line.trim_start().starts_with("//") {
                body.push_str(line);
                body.push('\n');
            }
        }
        bodies.push((current, body));
        for (name, body) in bodies {
            if !body.contains("phototux_canvas::write_layer_rgba(")
                && !body.contains("phototux_canvas::write_mask_r8(")
            {
                continue;
            }
            if PRESENTS_ANOTHER_WAY.contains(&name) {
                continue;
            }
            checked += 1;
            assert!(
                body.contains("self.recomposite()"),
                "`{name}` writes pixels to the GPU without calling \
                 `recomposite` — the canvas will keep showing the composite \
                 from before the edit"
            );
        }
        assert!(
            checked > 8,
            "found {checked} pixel writers — the scan broke rather than the shell"
        );
    }

    /// The export dialog does not keep its own list of formats.
    ///
    /// `RasterFormat::ALL` is the vocabulary; `exportNameFiltersJson`
    /// publishes it. A second list written into `Main.qml` had gone stale in
    /// the quiet direction — four of the six formats the writer handles — so
    /// BMP and GIF could be opened and never saved again, and nothing failed.
    #[test]
    fn the_export_dialog_takes_its_formats_from_the_engine() {
        let qml = std::fs::read_to_string(qml_dir().join("Main.qml")).expect("read Main.qml");
        let export = qml
            .split("id: exportFileDialog")
            .nth(1)
            .expect("Main.qml declares exportFileDialog");
        let body = export.split("\n    }").next().unwrap_or(export);
        assert!(
            body.contains("nameFilters: root.exportNameFilters"),
            "the export dialog binds `nameFilters` to something other than the \
             published list — a hand-written one goes stale the moment a \
             format is added to `RasterFormat`"
        );
        for stale in ["PNG images (", "JPEG images (", "WebP images ("] {
            assert!(
                !body.contains(stale),
                "the export dialog still spells out `{stale}…` — that list \
                 belongs to `RasterFormat`"
            );
        }
    }

    /// The modified flag is written in one place.
    ///
    /// `dirty` has two published views — the `dirty` property, which the
    /// window title and the close prompt bind to, and the `dirty` field inside
    /// `documentTabsJson`, which the tab strip is handed. A write that
    /// publishes one and not the other leaves them disagreeing, and they
    /// disagreed in the direction that matters: a freshly opened `.ptx` showed
    /// an unsaved marker on its tab, and `Ctrl+W` closed it with no prompt,
    /// because the flag the prompt reads was the correct one.
    #[test]
    fn the_modified_flag_is_written_in_one_place() {
        let source = include_str!("lib.rs");
        let writes: Vec<(usize, &str)> = source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.trim_start().starts_with("self.dirty = "))
            .map(|(i, line)| (i + 1, line.trim()))
            .collect();
        assert_eq!(
            writes.len(),
            1,
            "`self.dirty` is written at {} places: {writes:?} — every write goes \
             through `set_dirty`, which publishes the property *and* the tab \
             strip",
            writes.len()
        );
        let setter = source
            .split("fn set_dirty(&mut self, dirty: bool) {")
            .nth(1)
            .expect("lib.rs declares `set_dirty`");
        let body = setter.split("\n    }").next().unwrap_or(setter);
        assert!(
            body.contains("self.dirty = dirty;")
                && body.contains("self.dirty_changed();")
                && body.contains("self.refresh_document_tabs_json();"),
            "`set_dirty` no longer publishes both views of the flag"
        );
    }

    /// Every animation answers to the reduced-motion preference.
    ///
    /// "Reduced motion" is an accessibility preference, not a taste one: a
    /// user who sets it has asked the shell to stop moving. It reached the
    /// slider's scale and the toast fade and not the scroll bar, which went on
    /// growing and fading in the corner of the eye — the exact motion the
    /// preference exists to stop.
    ///
    /// An animation satisfies this by sitting in a `Behavior` that is
    /// `enabled: !Theme.reducedMotion`, or by reading the flag in its own
    /// duration.
    #[test]
    fn every_animation_answers_to_reduced_motion() {
        let mut checked = 0;
        for entry in std::fs::read_dir(qml_dir()).expect("read qml dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("qml") {
                continue;
            }
            let qml = std::fs::read_to_string(&path).expect("read qml");
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            for (i, line) in qml.lines().enumerate() {
                let trimmed = line.trim_start();
                // A declaration, not a reference: `loops: Animation.Infinite`
                // names the enum and animates nothing on its own.
                let declares = (trimmed.contains("Animation") || trimmed.contains("Animator"))
                    && trimmed.contains('{')
                    && !trimmed.contains("Animation.");
                if !declares || trimmed.starts_with("//") {
                    continue;
                }
                // A `FrameAnimation` is the shell's clock, not a transition:
                // it polls the file worker and measures frame rate, and
                // stopping it would stop those. What it *publishes* answers to
                // the preference instead — the phase driving the selection's
                // marching ants holds still.
                if line.contains("FrameAnimation") {
                    continue;
                }
                checked += 1;
                // Two lines above — the `Behavior on …` header and its
                // `enabled:` — and four below, for a flag read inside the
                // animation's own body. Deliberately tight: a wider window
                // reaches the *previous* `Behavior`'s `enabled:` line and
                // passes an animation that has none of its own, which is how
                // the first draft of this test missed the scroll bar.
                let from = i.saturating_sub(2);
                let window = qml
                    .lines()
                    .skip(from)
                    .take(i - from + 5)
                    .collect::<Vec<_>>();
                assert!(
                    window.iter().any(|l| l.contains("Theme.reducedMotion")),
                    "{name}:{} animates without consulting `Theme.reducedMotion`",
                    i + 1
                );
            }
        }
        assert!(
            checked >= 4,
            "found {checked} animations — the scan broke rather than the shell"
        );
    }

    /// A handler reacting to a host signal does not call the host back.
    ///
    /// An `AppSession` notify signal is emitted while the session is still
    /// mutably borrowed. A QML handler that reacts to one and calls a slot
    /// synchronously re-enters that borrow, and qtbridge answers a
    /// `BorrowConflict` by aborting the process — not by returning an error
    /// something could catch. Two blockers came from exactly this: tearing a
    /// panel off (T-027) and opening the Filter Gallery (T-028). The rule is
    /// `root.afterHostSlot`, which defers to the next turn of the event loop.
    ///
    /// This holds the crisply checkable half — the eleven
    /// `Connections { target: AppSession }` blocks. It does not reach a
    /// reactive handler written as a plain `on…Changed:` on some other object,
    /// which is the shape T-028 took; that one is guarded by
    /// `refreshShortcutYield` deferring inside the function rather than at its
    /// call sites.
    #[test]
    fn a_handler_for_a_host_signal_does_not_call_the_host_back() {
        let mut blocks = 0;
        for entry in std::fs::read_dir(qml_dir()).expect("read qml dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("qml") {
                continue;
            }
            let qml = std::fs::read_to_string(&path).expect("read qml");
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            let lines: Vec<&str> = qml.lines().collect();
            let mut i = 0;
            while i < lines.len() {
                if !lines[i].contains("Connections {")
                    || !lines[i..(i + 3).min(lines.len())]
                        .iter()
                        .any(|l| l.contains("target: AppSession"))
                {
                    i += 1;
                    continue;
                }
                blocks += 1;
                let mut depth = 0_i32;
                let mut j = i;
                while j < lines.len() {
                    depth += lines[j].matches('{').count() as i32;
                    depth -= lines[j].matches('}').count() as i32;
                    let call = lines[j].split("AppSession.").skip(1).any(|rest| {
                        rest.split(['.', ' ', ')', ','])
                            .next()
                            .is_some_and(|t| t.ends_with('('))
                    });
                    if call {
                        let deferred = lines[j.saturating_sub(2)..=j]
                            .iter()
                            .any(|l| l.contains("afterHostSlot"));
                        assert!(
                            deferred,
                            "{name}:{} calls a host slot from inside a \
                             `Connections {{ target: AppSession }}` block — that \
                             re-enters the borrow the signal was emitted under \
                             and aborts the process. Defer it through \
                             `root.afterHostSlot`",
                            j + 1
                        );
                    }
                    if depth == 0 && j > i {
                        break;
                    }
                    j += 1;
                }
                i = j + 1;
            }
        }
        assert!(
            blocks >= 8,
            "found {blocks} AppSession Connections blocks — the scan broke \
             rather than the shell"
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
    /// Every focusable shared control has to draw where keyboard focus is.
    ///
    /// `ChromeIconToolButton` did not, and it is the type that needs it most:
    /// icon-only, unlabelled, and used for the toolbar's Undo and Redo and for
    /// every panel header's actions. Those buttons are in the tab chain — Qt
    /// Quick Controls give `Button` `activeFocusOnTab` by default — so tabbing
    /// across the chrome moved focus through them with nothing on screen
    /// saying so. AT-SPI reported the focus moving from "Redo" to "About
    /// PhotoTux" while a pixel diff of the whole window found no change at all.
    ///
    /// The listed types are the shared controls a keyboard reaches. The ones
    /// left out are left out for a reason: `ThemedMenuItem` is driven by
    /// `highlighted` rather than focus, because a menu moves a highlight and
    /// not the focus; `ThemedScrollBar`, `ThemedBusyIndicator`, `ThemedIcon`,
    /// `ThemedToolTip`, `ThemedMenu` and the two dialog bars take no focus of
    /// their own.
    #[test]
    fn every_focusable_control_draws_its_focus() {
        const FOCUSABLE: [&str; 8] = [
            "ChromeIconToolButton.qml",
            "ThemedButton.qml",
            "ThemedCheckBox.qml",
            "ThemedComboBox.qml",
            "ThemedSlider.qml",
            "ThemedSpinBox.qml",
            "ThemedTextField.qml",
            "DisclosureGroup.qml",
        ];
        let mut checked = 0;
        for component in FOCUSABLE {
            let text = std::fs::read_to_string(qml_dir().join(component))
                .unwrap_or_else(|e| panic!("{component}: {e}"));
            let draws = text.lines().any(|line| {
                let line = line.trim_start();
                !line.starts_with("//")
                    && (line.contains("visualFocus") || line.contains("activeFocus"))
            });
            checked += 1;
            assert!(
                draws,
                "{component} never reads visualFocus or activeFocus, so keyboard \
                 focus lands on it invisibly — give it a Theme.focusRing border"
            );
        }
        assert_eq!(
            checked,
            FOCUSABLE.len(),
            "the scan broke rather than the shell"
        );
    }

    /// The same requirement for the icon buttons written out in place.
    ///
    /// Eleven `ToolButton`s across the shell hand-roll their own background —
    /// the panel headers, the options bar's mode and align runs, the layer
    /// visibility eye, the tool-strip overflow, two dialog close buttons — and
    /// every one of them was drawing hover and checked but not focus. They are
    /// the same control as `ChromeIconToolButton` wearing a different padding,
    /// so they answer the same question.
    #[test]
    fn every_icon_button_draws_its_focus() {
        let mut checked = 0;
        for (name, text) in qml_files() {
            for start in instantiations_of(&text, "ToolButton") {
                let body = body_at_line(&text, start);
                checked += 1;
                assert!(
                    body.contains("visualFocus"),
                    "{name}:{start} draws hover and checked but not focus, so a \
                     keyboard lands on it invisibly — add a Theme.focusRing \
                     border on visualFocus"
                );
            }
        }
        assert!(
            checked >= 10,
            "found {checked} tool buttons — the scan broke rather than the shell"
        );
    }
    /// A close or quit deferred for a save is either finished or abandoned.
    ///
    /// Answering the unsaved-changes prompt with Save wrote the file and then
    /// stopped: the document stayed open, and File ▸ Quit ▸ Save left the
    /// application running. The prompt's button handler cannot do the close
    /// itself — a save goes out to the file worker and lands later — so the
    /// action is parked in `pendingDestructiveAction` and the shell has to
    /// pick it up again.
    ///
    /// Both halves are pinned here because either one alone is a bug. Without
    /// the resume, Save does not close. Without the abandon, backing out of
    /// the file dialog leaves the action armed against the *next* successful
    /// save, so an ordinary Ctrl+S half an hour later closes the document the
    /// user is still working in.
    #[test]
    fn a_close_deferred_for_a_save_is_finished_or_abandoned() {
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
            .expect("the host crate is readable from its own tests");
        let at = source
            .find("fn handle_file_saved(&mut self, path: PathBuf)")
            .expect("handle_file_saved is there to read");
        let line = source[..at].matches('\n').count() + 1;
        let body = body_at_line(&source, line);
        assert!(
            body.contains("mark_persisted("),
            "the slice missed the function body — the scan broke rather than \
             the host"
        );
        assert!(
            body.contains("self.document_saved()"),
            "handle_file_saved does not emit documentSaved, so nothing tells \
             the shell a save landed and a close answered with Save never \
             happens"
        );

        let shell = qml_files()
            .into_iter()
            .find(|(name, _)| name == "Main.qml")
            .map(|(_, text)| text)
            .expect("Main.qml");
        assert!(
            shell.contains("function onDocumentSaved()"),
            "Main.qml never handles documentSaved, so a close or quit \
             answered with Save saves and then stops"
        );
        let dialog_at = shell
            .find("id: saveFileDialog")
            .expect("saveFileDialog is there to read");
        let dialog_line = shell[..dialog_at].matches('\n').count();
        let dialog = body_at_line(&shell, dialog_line);
        assert!(
            dialog.contains("pendingDestructiveAction = \"\""),
            "saveFileDialog does not clear pendingDestructiveAction when it is \
             dismissed, so backing out of the dialog arms the close against \
             the next save that succeeds"
        );
    }
    /// Chrome takes its colours from `Theme.qml`, not from the point of use.
    ///
    /// Six canvas overlays were eight-digit literals — the grid, a guide, the
    /// selection preview, the marching-ants stroke, the crop wash and the
    /// Navigator's checkerboard — which is a second palette by any other name,
    /// and exactly where Qt's `#AARRGGBB` order is invisible. The crop wash had
    /// once shipped as a pale green fill inside a cyan border because the alpha
    /// was read as the red channel. A token named once in `Theme.qml` is where
    /// that mistake is legible; a literal beside `Rectangle` is where it is not.
    ///
    /// Document colours are not chrome and are excepted by name: the swatch
    /// palette the user paints with, and the fallbacks a shape's fill and
    /// stroke rows show before the layer has one. Both are values that belong
    /// to the artwork, and theming them would change what the file contains.
    #[test]
    fn chrome_colours_come_from_the_theme() {
        const DOCUMENT_COLOURS: [(&str, &str); 2] = [
            ("Main.qml", "model: ["),
            ("PropertiesPanel.qml", "hex: root.shape."),
        ];
        let mut checked = 0;
        for (name, text) in qml_files() {
            if name == "Theme.qml" {
                continue;
            }
            for (number, line) in text.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("///") {
                    continue;
                }
                let Some(at) = line.find("\"#") else { continue };
                let rest = &line[at + 2..];
                let digits = rest.chars().take_while(char::is_ascii_hexdigit).count();
                if !matches!(digits, 3 | 6 | 8) || !rest[digits..].starts_with('"') {
                    continue;
                }
                checked += 1;
                let allowed = DOCUMENT_COLOURS.iter().any(|(file, marker)| {
                    *file == name
                        && text
                            .lines()
                            .skip(number.saturating_sub(6))
                            .take(7)
                            .any(|near| near.contains(marker))
                });
                assert!(
                    allowed,
                    "{name}:{} paints chrome from a colour literal — name it in \
                     Theme.qml instead, where an #AARRGGBB alpha is legible",
                    number + 1
                );
            }
        }
        assert!(
            checked >= 5,
            "found {checked} colour literals — the scan broke rather than the shell"
        );
    }
    /// A refused command reaches the user through `report_action_error`.
    ///
    /// `CommandError`'s `Display` is a log line: it puts "command rejected: "
    /// in front of the reason, which is scaffolding a status bar should never
    /// show. `user_message` exists for exactly this — a capital, a full stop
    /// and none of that — and `report_action_error` adds the two things a
    /// refusal also needs: the Warning level, because the command *did not
    /// happen*, and the announcement that carries it to assistive technology.
    ///
    /// Four call sites rendered the error themselves instead: three in the
    /// filter gallery and one on the selection path, all of them saying
    /// "command rejected: …" out loud.
    #[test]
    fn a_refused_command_is_reported_not_rendered() {
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
            .expect("the host crate is readable from its own tests");
        let lines: Vec<&str> = source.lines().collect();
        for (number, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            if !(trimmed.starts_with("self.notify(") && line.contains("error.to_string()")) {
                continue;
            }
            // `DocumentError` is excepted, and only where the value in hand is
            // one: its messages are already sentences a person can read — "no
            // document is open", "1024 px is not a usable document edge" —
            // with none of `CommandError`'s log scaffolding in front of them.
            let from_document_error = lines[number.saturating_sub(3)..number]
                .iter()
                .any(|near| near.contains("DocumentError"));
            assert!(
                from_document_error,
                "lib.rs:{} hands a rendered error to notify — call \
                 report_action_error(&error), which classifies it, drops the \
                 \"command rejected\" scaffolding and announces it",
                number + 1
            );
        }
        assert!(
            source.contains("fn report_action_error(&mut self, error: &CommandError)"),
            "report_action_error is gone — the scan broke rather than the host"
        );
    }
    /// Every dock seam has a ceiling that knows about the dock.
    ///
    /// `PanelResizeGrip` clamped at a constant 2000, mirroring
    /// `DockTopology::MAX_PANEL_HEIGHT`, and neither side subtracted what the
    /// panels below the seam needed — so one drag to the bottom of the screen
    /// made the panel above fill the dock and every group under it vanish,
    /// with the Window menu still listing them as visible.
    ///
    /// Both halves are pinned, because leaving either out is the same bug: the
    /// helper that computes the budget, and the binding at every seam that
    /// uses it. A missing binding is silent — `maximumHeight` is an `int`, so
    /// an undefined value reads as 0 and the grip falls back to its absolute
    /// bound — which is exactly how this was first "fixed" without taking
    /// effect.
    #[test]
    fn every_dock_seam_is_clamped_against_the_dock() {
        let shell = qml_files()
            .into_iter()
            .find(|(name, _)| name == "Main.qml")
            .map(|(_, text)| text)
            .expect("Main.qml");
        assert!(
            shell.contains("function panelMaxHeight(panelId, dockHeight)"),
            "Main.qml has no panelMaxHeight, so a seam has no budget to clamp \
             against and the panels below it can be pushed off the dock"
        );
        let mut checked = 0;
        for start in instantiations_of(&shell, "PanelResizeGrip") {
            let body = body_at_line(&shell, start);
            checked += 1;
            assert!(
                body.contains("maximumHeight: root.panelMaxHeight("),
                "Main.qml:{start} leaves a seam on the absolute 2000 ceiling — \
                 bind maximumHeight to root.panelMaxHeight so the stack below \
                 keeps its room"
            );
        }
        assert!(
            checked >= 5,
            "found {checked} seams — the scan broke rather than the dock"
        );
    }
}
