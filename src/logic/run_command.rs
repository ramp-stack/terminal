use quartz::Shared;
use crate::terminal::Terminal;
use crate::terminal::line::Line;
use crate::terminal::state::spawn_streaming;

pub fn run_command(raw_cmd: &str, t: &Terminal, cwd: &Shared<String>) {
    let cmd = raw_cmd.trim();
    if cmd.is_empty() { return; }

    {
        let mut st = t.state.get_mut();
        if st.history.last().map(|s| s.as_str()) != Some(cmd) {
            st.history.push(cmd.to_string());
        }
        st.history_idx = None;
    }

    if cmd == "clear" || cmd == "cls" { t.clear(); return; }
    if cmd == "pwd" { t.push(Line::output(cwd.get().clone())); return; }

    if let Some(rest) = cmd.strip_prefix("cd") {
        let dir     = rest.trim();
        let current = cwd.get().clone();
        let target  = if dir.is_empty() || dir == "~" {
            std::env::var("HOME").unwrap_or(current.clone())
        } else if dir.starts_with('/') {
            dir.to_string()
        } else {
            format!("{current}/{dir}")
        };
        match std::fs::canonicalize(&target) {
            Ok(p) if p.is_dir() => {
                *cwd.get_mut() = p.to_string_lossy().into_owned();
                t.push(Line::output(format!("~ {}", p.to_string_lossy())));
            }
            Ok(_)  => t.push(Line::error(format!("not a directory: {target}"))),
            Err(e) => t.push(Line::error(format!("cd: {e}"))),
        }
        return;
    }

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let wd    = cwd.get().clone();
    let mut st = t.state.get_mut();
    match spawn_streaming(&shell, cmd, &wd) {
        Ok((rx, child)) => {
            st.stream_rx = Some(rx);
            st.child     = Some(child);
            st.running   = true;
        }
        Err(e) => {
            st.push(Line::error(e));
        }
    }
}