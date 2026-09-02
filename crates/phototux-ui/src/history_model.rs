//! The history panel's list model.
//!
//! The same three-strings-for-one-list shape the layers panel had, one size
//! down: labels, kinds and entry ids each crossed to QML as a pipe-joined
//! string, and the delegate re-associated them by index. Correct only while all
//! three were rebuilt together — and the id list is the one that matters, since
//! clicking a row hands its id straight to `jumpHistoryEntry`. A row whose
//! label had caught up but whose id had not would jump the document somewhere
//! the user did not click.
//!
//! Row-change planning is shared with [`crate::layer_model`] rather than
//! restated; only the item type differs.

use crate::layer_model::{RowUpdate, plan_update};
use qtbridge::{QListModelBase, QModelItem, qobject};
use std::cell::RefCell;
// The QModelItem derive expands to a role table and needs HashMap in scope at
// the use site.
use std::collections::HashMap;
use std::rc::Rc;

/// One history entry, as QML sees it.
///
/// Field names are role names, verbatim and un-camel-cased — see
/// [`crate::layer_model::LayerItem`].
#[derive(QModelItem, Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryItem {
    pub label: String,
    /// `graph`, `stroke`, `selection` or `transform`.
    pub kind: String,
    /// Signed because QML has no unsigned integer type, and this is handed
    /// back to `jumpHistoryEntry` unchanged.
    pub entry_id: i64,
    /// The step is undone, and clicking it redoes forward to it.
    pub undone: bool,
}

impl From<phototux_engine::HistoryRow> for HistoryItem {
    fn from(row: phototux_engine::HistoryRow) -> Self {
        Self {
            label: row.label,
            kind: row.kind,
            entry_id: row.entry_id,
            undone: row.undone,
        }
    }
}

#[qobject(Base = QListModel, NoQmlElement)]
mod history_list {
    use super::HistoryItem;
    use qtbridge::QListModel;

    /// The history rows QML draws, newest first.
    #[derive(Default)]
    pub struct HistoryListModel {
        pub(crate) rows: Vec<HistoryItem>,
    }

    impl QListModel for HistoryListModel {
        type Item = HistoryItem;

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

pub use history_list::HistoryListModel;

/// Bring the model's rows to `next`, telling Qt as little as it needs.
///
/// History grows by one at the newest end, which is a length change and so a
/// rebuild — but an edit that only relabels the tip repaints one row.
pub fn apply_rows(model: &Rc<RefCell<HistoryListModel>>, next: Vec<HistoryItem>) {
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

    #[test]
    fn an_engine_row_survives_the_crossing_intact() {
        let row = phototux_engine::HistoryRow {
            entry_id: 42,
            label: "Paint stroke".to_owned(),
            kind: "stroke".to_owned(),
            undone: true,
        };
        let item = HistoryItem::from(row.clone());
        assert_eq!(item.entry_id, row.entry_id);
        assert_eq!(item.label, row.label);
        assert_eq!(item.kind, row.kind);
        assert_eq!(item.undone, row.undone);
    }

    /// The delegate binds these by hand, and `entry_id` is the one that has
    /// consequences: it is passed to `jumpHistoryEntry`, so a wrong or missing
    /// role moves the document to the wrong point in time rather than merely
    /// drawing the wrong text.
    #[test]
    fn role_names_are_the_ones_qml_reads() {
        let mut names: Vec<String> = <HistoryItem as qtbridge::QModelItem>::role_names()
            .into_values()
            .collect();
        names.sort();
        assert_eq!(names, vec!["entry_id", "kind", "label", "undone"]);
    }
}
