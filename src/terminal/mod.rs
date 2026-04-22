pub mod ansi;
pub mod line;
pub mod state;
pub mod text;

use quartz::Shared;
use crate::preferences::TermSettings;
use crate::terminal::line::Line;
use crate::terminal::state::State;

pub use line::{Line as TermLine, LineKind};

#[derive(Clone)]
pub struct Terminal {
    pub settings:      Shared<TermSettings>,
    pub(crate) state:  Shared<State>,
}

impl Terminal {
    pub fn push(&self, line: Line) { self.state.get_mut().push(line); }
    pub fn push_many(&self, lines: impl IntoIterator<Item = Line>) {
        let mut st = self.state.get_mut();
        for l in lines { st.push(l); }
    }
    pub fn clear(&self) {
        let mut st = self.state.get_mut();
        st.buf.clear(); st.h_scroll = 0.0; st.dirty = true;
    }
    pub fn is_running(&self) -> bool { self.state.get().running }
}