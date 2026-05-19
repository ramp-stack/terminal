use quartz::Shared;
use crate::preferences::TermSettings;
use crate::line::Line;
use crate::state::State;

#[derive(Clone)]
pub struct Terminal {
    pub settings:     Shared<TermSettings>,
    pub(crate) state: Shared<State>,
}

impl Terminal {
    pub fn push(&self, line: Line) {
        self.state.get_mut().push(line);
    }
    pub fn push_many(&self, lines: impl IntoIterator<Item = Line>) {
        let mut st = self.state.get_mut();
        for l in lines { st.push(l); }
    }
    pub fn clear(&self) {
        let mut st = self.state.get_mut();
        st.buf.clear();
        st.scroll       = 0.0;
        st.scroll_vel   = 0.0;
        st.h_scroll     = 0.0;
        st.h_scroll_vel = 0.0;
        st.slot_count   = 0;
        st.dirty        = true;
    }
    pub fn is_running(&self) -> bool {
        self.state.get().running
    }
    pub fn set_cwd(&self, cwd: String) {
        self.state.get_mut().cwd = cwd;
    }
    pub fn cwd(&self) -> String {
        self.state.get().cwd.clone()
    }
}