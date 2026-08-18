use rataflow::{Flow, FlowSnapshot};

pub struct History {
    current: FlowSnapshot,
    undo_stack: Vec<FlowSnapshot>,
    redo_stack: Vec<FlowSnapshot>,
}

impl History {
    pub fn new(flow: &Flow) -> Self {
        Self {
            current: flow.to_snapshot(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, flow: &Flow) {
        self.undo_stack.push(self.current.clone());
        self.current = flow.to_snapshot();
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, flow: &mut Flow) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            let viewport = flow.viewport;
            self.redo_stack
                .push(std::mem::replace(&mut self.current, prev));
            *flow = Flow::from_snapshot(self.current.clone()).expect("valid snapshot");
            flow.viewport = viewport;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, flow: &mut Flow) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let viewport = flow.viewport;
            self.undo_stack
                .push(std::mem::replace(&mut self.current, next));
            *flow = Flow::from_snapshot(self.current.clone()).expect("valid snapshot");
            flow.viewport = viewport;
            true
        } else {
            false
        }
    }
}
