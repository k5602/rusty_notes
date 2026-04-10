use super::cursor::Cursor;
use super::text::Text;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub text: Text,
    pub cursor: Cursor,
}

#[derive(Debug)]
pub struct UndoStack {
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    max_size: usize,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size: 100,
        }
    }

    pub fn push(&mut self, snapshot: Snapshot) {
        if let Some(top) = self.undo_stack.last() {
            if top.text.lines == snapshot.text.lines {
                return;
            }
        }
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let popped = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(popped)
    }

    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let popped = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(popped)
    }
}
