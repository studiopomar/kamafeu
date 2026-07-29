use crate::project::model::UProject;

pub struct UndoManager {
    undo_stack: Vec<UProject>,
    redo_stack: Vec<UProject>,
    max_history: usize,
}

impl UndoManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    pub fn push_state(&mut self, current: UProject) {
        self.undo_stack.push(current);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: UProject) -> Option<UProject> {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current);
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self, current: UProject) -> Option<UProject> {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(current);
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
        Self::new(50)
    }
}
