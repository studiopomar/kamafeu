use crate::project::model::UProject;
use std::collections::VecDeque;

pub struct UndoManager {
    undo_stack: VecDeque<UProject>,
    redo_stack: Vec<UProject>,
    max_history: usize,
}

impl UndoManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_history.min(100)),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    pub fn push_state(&mut self, current: UProject) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(current);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: UProject) -> Option<UProject> {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push(current);
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self, current: UProject) -> Option<UProject> {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push_back(current);
            Some(next)
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(40)
    }
}

#[cfg(test)]
mod tests {
    use super::UndoManager;
    use crate::project::model::{UNote, UProject};

    #[test]
    fn one_creation_gesture_undoes_and_redoes_the_complete_note() {
        let before = UProject::default();
        let mut after = before.clone();
        after.parts[0]
            .notes
            .push(UNote::new("ka", "C4", 120.0, 480.0));

        let mut history = UndoManager::new(10);
        history.push_state(before);

        let undone = history.undo(after.clone()).unwrap();
        assert!(undone.parts[0].notes.is_empty());

        let redone = history.redo(undone).unwrap();
        assert_eq!(redone.parts[0].notes.len(), 1);
        assert_eq!(redone.parts[0].notes[0].duration_ms, 480.0);
    }
}
