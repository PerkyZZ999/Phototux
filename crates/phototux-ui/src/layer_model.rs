//! The layers panel's list model.
//!
//! QML used to receive the layer stack as six pipe-joined strings and an
//! integer count. Each one was rebuilt whenever anything about any layer
//! changed, `split("|")` in QML produced six fresh JavaScript arrays, and every
//! delegate indexed all six by hand — after inverting its own row number,
//! because the strings ran bottom→top while the panel draws top→bottom.
//!
//! Nothing enforced that the six agreed. A delegate reading past the end of one
//! of them fell back to a default that looked like real data: a visible,
//! unselected, unmasked raster layer. Renaming a layer rebuilt and re-announced
//! all six, so the whole panel's bindings re-evaluated to redraw one label.
//!
//! A real model replaces all of that. Rows arrive already in display order with
//! their fields together, Qt delivers each row to its own delegate, and adding a
//! per-layer field is one role rather than a seventh string to keep aligned.
//! Deciding what a row *contains* stays in `phototux_engine::layer_rows`, which
//! is testable without a Qt session; this file is only the Qt shape of it.

use qtbridge::{QListModelBase, QModelItem, qobject};
use std::cell::RefCell;
// The QModelItem derive expands to a role table and needs HashMap in scope at
// the use site rather than importing it itself.
use std::collections::HashMap;
use std::rc::Rc;

/// One layer, as QML sees it.
///
/// The derive turns each field into a role named after it, so the delegate
/// writes `required property string name` and gets this field. Field names are
/// therefore QML API: renaming one silently breaks the delegate that reads it,
/// which is what [`crate::layer_model::tests`] guards.
#[derive(QModelItem, Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerItem {
    pub name: String,
    pub kind: String,
    /// Not `visible`: the role name becomes a property on the delegate, and
    /// `Item.visible` already exists there. A `required property bool visible`
    /// would shadow the delegate's own visibility, so the layer's flag carries
    /// a name of its own.
    pub layer_visible: bool,
    /// `0` none, `1` mask enabled, `2` mask disabled — see
    /// [`phototux_engine::LayerRow::mask_flag`].
    pub mask_flag: i32,
    pub clips_to_below: bool,
    pub selected: bool,
    pub active: bool,
    /// The engine-order index this row came from, for commands that take one.
    pub stack_index: i32,
}

impl From<phototux_engine::LayerRow> for LayerItem {
    fn from(row: phototux_engine::LayerRow) -> Self {
        Self {
            name: row.name,
            kind: row.kind,
            layer_visible: row.visible,
            mask_flag: row.mask_flag,
            clips_to_below: row.clips_to_below,
            selected: row.selected,
            active: row.active,
            stack_index: row.stack_index,
        }
    }
}

#[qobject(Base = QListModel, NoQmlElement)]
mod layer_list {
    use super::LayerItem;
    use qtbridge::QListModel;

    /// The layer rows QML draws.
    ///
    /// Owned by `AppSession` and handed to QML as a property rather than
    /// registered as its own QML type: there is exactly one layer list, it
    /// belongs to the session that maintains it, and letting QML instantiate a
    /// second empty one would only give a panel a way to show nothing.
    #[derive(Default)]
    pub struct LayerListModel {
        pub(crate) rows: Vec<LayerItem>,
    }

    impl QListModel for LayerListModel {
        type Item = LayerItem;

        fn len(&self) -> usize {
            self.rows.len()
        }

        fn get(&self, index: usize) -> Option<&Self::Item> {
            self.rows.get(index)
        }

        fn set_unnotified(&mut self, index: usize, value: Self::Item) -> bool {
            match self.rows.get_mut(index) {
                Some(slot) => {
                    *slot = value;
                    true
                }
                None => false,
            }
        }

        fn reset_unnotified(&mut self) {
            self.rows.clear();
        }

        fn push_unnotified(&mut self, value: Self::Item) {
            self.rows.push(value);
        }

        fn insert_unnotified(&mut self, index: usize, value: Self::Item) {
            self.rows.insert(index.min(self.rows.len()), value);
        }

        fn pop_unnotified(&mut self) -> Option<Self::Item> {
            self.rows.pop()
        }

        fn remove_unnotified(&mut self, index: usize) -> Self::Item {
            self.rows.remove(index)
        }
    }
}

pub use layer_list::LayerListModel;

/// What Qt has to be told about a change to the rows.
///
/// The distinction is what the user sees. A reset makes every view drop its
/// delegates and rebuild them, losing scroll position and any in-progress
/// rename; a per-row write leaves the delegates in place and repaints one line.
/// Most layer edits — renaming, toggling visibility, changing the selection —
/// keep the row count the same, so the common case is deliberately the cheap
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowUpdate {
    /// Nothing moved; say nothing. Layer state resyncs on every graph
    /// revision, so this is the most frequent outcome by far.
    Unchanged,
    /// The row count changed, so existing delegates no longer correspond to
    /// anything and a reset is the honest signal.
    Rebuild,
    /// These row indices have new content.
    Rows(Vec<usize>),
}

/// Decide what changed between two row lists.
///
/// Split out from [`apply_rows`] so the decision is testable: executing it
/// needs an attached `QObject`, but choosing between "say nothing", "repaint
/// these three rows" and "rebuild" is ordinary logic, and getting it wrong is
/// either a stale panel or a panel that flickers on every keystroke.
#[must_use]
pub fn plan_update(previous: &[LayerItem], next: &[LayerItem]) -> RowUpdate {
    if previous.len() != next.len() {
        return RowUpdate::Rebuild;
    }
    let changed: Vec<usize> = next
        .iter()
        .enumerate()
        .filter(|(i, item)| previous.get(*i) != Some(*item))
        .map(|(i, _)| i)
        .collect();
    if changed.is_empty() {
        RowUpdate::Unchanged
    } else {
        RowUpdate::Rows(changed)
    }
}

/// Bring the model's rows to `next`, telling Qt as little as it needs.
pub fn apply_rows(model: &Rc<RefCell<LayerListModel>>, next: Vec<LayerItem>) {
    let plan = plan_update(&model.borrow().rows, &next);
    match plan {
        RowUpdate::Unchanged => {}
        RowUpdate::Rebuild => {
            model.borrow_mut().reset();
            for item in next {
                model.borrow_mut().push(item);
            }
        }
        RowUpdate::Rows(indices) => {
            for index in indices {
                if let Some(item) = next.get(index) {
                    model.borrow_mut().set(index, item.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, stack_index: i32) -> LayerItem {
        LayerItem {
            name: name.to_owned(),
            kind: "raster".to_owned(),
            layer_visible: true,
            stack_index,
            ..Default::default()
        }
    }

    #[test]
    fn an_unchanged_list_says_nothing() {
        let rows = vec![row("a", 1), row("b", 0)];
        assert_eq!(plan_update(&rows, &rows.clone()), RowUpdate::Unchanged);
    }

    #[test]
    fn a_different_length_forces_a_rebuild() {
        let one = vec![row("a", 0)];
        let two = vec![row("a", 1), row("b", 0)];
        assert_eq!(plan_update(&one, &two), RowUpdate::Rebuild);
        assert_eq!(plan_update(&two, &one), RowUpdate::Rebuild);
        assert_eq!(plan_update(&[], &one), RowUpdate::Rebuild);
    }

    /// Renaming one layer must repaint one line, not rebuild the panel — a
    /// rebuild mid-rename would drop the editor the user is typing into.
    #[test]
    fn one_edited_row_reports_only_that_row() {
        let before = vec![row("a", 2), row("b", 1), row("c", 0)];
        let mut after = before.clone();
        after[1].name = "renamed".to_owned();
        assert_eq!(plan_update(&before, &after), RowUpdate::Rows(vec![1]));
    }

    #[test]
    fn several_edited_rows_are_reported_together() {
        let before = vec![row("a", 2), row("b", 1), row("c", 0)];
        let mut after = before.clone();
        after[0].selected = true;
        after[2].layer_visible = false;
        assert_eq!(plan_update(&before, &after), RowUpdate::Rows(vec![0, 2]));
    }

    /// Every role participates in the comparison. A field that changed without
    /// being noticed here is a panel showing yesterday's state, and the ones
    /// most likely to be forgotten are the flags rather than the name.
    #[test]
    fn every_field_counts_as_a_change() {
        let base = row("a", 0);
        let mutations: Vec<fn(&mut LayerItem)> = vec![
            |r| r.name = "other".to_owned(),
            |r| r.kind = "text".to_owned(),
            |r| r.layer_visible = !r.layer_visible,
            |r| r.mask_flag = 2,
            |r| r.clips_to_below = !r.clips_to_below,
            |r| r.selected = !r.selected,
            |r| r.active = !r.active,
            |r| r.stack_index += 1,
        ];
        for (i, mutate) in mutations.iter().enumerate() {
            let mut after = base.clone();
            mutate(&mut after);
            assert_eq!(
                plan_update(std::slice::from_ref(&base), std::slice::from_ref(&after)),
                RowUpdate::Rows(vec![0]),
                "mutation {i} was not detected as a change"
            );
        }
    }

    #[test]
    fn an_engine_row_survives_the_crossing_intact() {
        let engine_row = phototux_engine::LayerRow {
            name: "Sky".to_owned(),
            kind: "raster".to_owned(),
            visible: false,
            mask_flag: 2,
            clips_to_below: true,
            selected: true,
            active: false,
            stack_index: 3,
        };
        let item = LayerItem::from(engine_row.clone());
        assert_eq!(item.name, engine_row.name);
        assert_eq!(item.kind, engine_row.kind);
        assert_eq!(item.layer_visible, engine_row.visible);
        assert_eq!(item.mask_flag, engine_row.mask_flag);
        assert_eq!(item.clips_to_below, engine_row.clips_to_below);
        assert_eq!(item.selected, engine_row.selected);
        assert_eq!(item.active, engine_row.active);
        assert_eq!(item.stack_index, engine_row.stack_index);
    }

    /// The delegate reads roles by field name, so these names are QML API.
    ///
    /// Note the case: the derive uses the Rust field name verbatim, and the
    /// `ConvertToCamelCase` option that camel-cases slots and properties does
    /// not reach model roles. The delegate therefore declares
    /// `required property int mask_flag`, not `maskFlag`. Renaming a field
    /// here breaks a QML file this crate cannot see, and nothing else would
    /// fail first — which is what this test is for.
    #[test]
    fn role_names_are_the_ones_qml_reads() {
        let mut names: Vec<String> = <LayerItem as qtbridge::QModelItem>::role_names()
            .into_values()
            .collect();
        names.sort();
        let mut expected = vec![
            "active",
            "clips_to_below",
            "kind",
            "layer_visible",
            "mask_flag",
            "name",
            "selected",
            "stack_index",
        ];
        expected.sort_unstable();
        assert_eq!(
            names, expected,
            "the layers delegate binds these role names by hand"
        );
    }
}
