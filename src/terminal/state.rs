use std::io::{BufRead, BufReader};
use std::process::Child;
use std::sync::mpsc::{self, Receiver};
use crate::terminal::line::Line;

pub(crate) enum StreamMsg { Stdout(String), Stderr(String), Done(Option<i32>) }

pub(crate) struct State {
    pub buf:          Vec<Line>,
    pub input:        String,
    pub cursor_col:   usize,
    pub scroll:       f32,
    pub scroll_vel:   f32,
    pub h_scroll:     f32,
    pub h_scroll_vel: f32,
    pub stream_rx:    Option<Receiver<StreamMsg>>,
    pub child:        Option<Child>,
    pub running:      bool,
    pub slot_count:   usize,
    pub dirty:        bool,
    pub scrollback:   usize,
    pub history:      Vec<String>,
    pub history_idx:  Option<usize>,
}

impl State {
    pub fn new(scrollback: usize) -> Self {
        Self {
            buf: Vec::new(), input: String::new(), cursor_col: 0,
            scroll: 0.0, scroll_vel: 0.0,
            h_scroll: 0.0, h_scroll_vel: 0.0,
            stream_rx: None, child: None, running: false,
            slot_count: 0, dirty: true, scrollback,
            history: Vec::new(),
            history_idx: None,
        }
    }

    pub fn kill_child(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        self.stream_rx = None;
        self.running   = false;
    }

    pub fn push(&mut self, line: Line) {
        self.buf.push(line);
        if self.buf.len() > self.scrollback { self.buf.remove(0); }
        self.dirty = true;
    }

    pub fn total_lines(&self) -> usize { self.buf.len() + 1 }
    pub fn content_h(&self, lh: f32)   -> f32 { self.total_lines() as f32 * lh }

    pub fn v_scroll_max(&self, lh: f32, vh: f32) -> f32 {
        (self.content_h(lh) - vh + lh).max(0.0)
    }

    pub fn clamp_v(&mut self, lh: f32, vh: f32) {
        self.scroll = self.scroll.clamp(0.0, self.v_scroll_max(lh, vh));
    }

    pub fn snap_bottom(&mut self, lh: f32, vh: f32) {
        self.scroll = self.v_scroll_max(lh, vh);
    }

    pub fn drain_stream(&mut self) -> bool {
        let msgs: Vec<StreamMsg> = match &self.stream_rx {
            None => return false,
            Some(rx) => {
                let mut v = Vec::new();
                loop {
                    match rx.try_recv() {
                        Ok(m)  => v.push(m),
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => { v.push(StreamMsg::Done(None)); break; }
                    }
                }
                v
            }
        };
        if msgs.is_empty() { return false; }
        for m in msgs {
            match m {
                StreamMsg::Stdout(l) => self.push(Line::output(l)),
                StreamMsg::Stderr(l) => self.push(Line::stderr(l)),
                StreamMsg::Done(code) => {
                    if let Some(c) = code { if c != 0 { self.push(Line::error(format!("exit {c}"))); } }
                    if let Some(mut c) = self.child.take() { let _ = c.wait(); }
                    self.stream_rx = None;
                    self.running   = false;
                }
            }
        }
        true
    }
}

pub(crate) fn spawn_streaming(shell: &str, cmd: &str, working_dir: &str)
    -> Result<(Receiver<StreamMsg>, Child), String>
{
    use std::process::{Command, Stdio};

    let mut child = Command::new(shell)
        .arg("-c").arg(cmd)
        .current_dir(working_dir)
        .env("TERM", "xterm-256color")
        .env("CARGO_TERM_COLOR", "always")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn: {e}"))?;

    let (tx, rx) = mpsc::channel::<StreamMsg>();

    let tx2    = tx.clone();
    let stdout = child.stdout.take().unwrap();
    let t1 = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if let Ok(l) = line { if tx2.send(StreamMsg::Stdout(l)).is_err() { break; } }
        }
    });

    let tx3    = tx.clone();
    let stderr = child.stderr.take().unwrap();
    let t2 = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            if let Ok(l) = line { if tx3.send(StreamMsg::Stderr(l)).is_err() { break; } }
        }
    });

    std::thread::spawn(move || {
        let _ = t1.join();
        let _ = t2.join();
        let _ = tx.send(StreamMsg::Done(None));
    });

    Ok((rx, child))
}

pub(crate) fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(s.len())
}