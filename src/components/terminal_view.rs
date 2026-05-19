use quartz::{Arc, Color, Font, GameObject};
use quartz::tint_overlay;
use quartz::NamedKey;
use quartz::Key;
use quartz::Canvas;
use quartz::{Text, Span, Align};
use quartz::Shared;
use ramp::Drawable;
use crate::preferences::TermSettings;
use crate::terminal::Terminal;
use crate::line::{Line, LineKind};
use crate::state::char_to_byte;
use crate::text::{make_plain_text, build_line_text};

const PROMPT_IDLE:    &str = "❯ ";
const PROMPT_RUNNING: &str = "● ";

const COL_PROMPT_IDLE:    Color = Color(200, 160, 255, 255);
const COL_PROMPT_RUNNING: Color = Color(210, 120,  60, 255);
const COL_ECHO:           Color = Color(100, 100, 100, 255);
const COL_CWD:            Color = Color( 90,  90, 110, 255);

const SCROLL_LINES: f32 = 3.0;

pub struct TerminalView;

impl TerminalView {
    pub fn setup(cv: &mut Canvas, s: &TermSettings) {
        cv.add_game_object("term_bg".into(),
            GameObject::build("term_bg").position(s.offset_x, s.offset_y)
                .size(1.0, 1.0).layer(0).image(tint_overlay(1.0, 1.0, s.bg)).finish());
        cv.add_game_object("term_cursor".into(),
            GameObject::build("term_cursor").position(-9999.0, -9999.0)
                .size(s.cw(), s.font_size).layer(6)
                .image(tint_overlay(s.cw(), s.font_size, Color(255, 255, 255, 180))).finish());
        cv.set_var("_term_cursor_blink_t",  quartz::Value::from(0.0f32));
        cv.set_var("_term_cursor_blink_on", quartz::Value::from(true));
        cv.set_var("_term_objects_exist",   quartz::Value::from(true));
    }

    pub fn register(
        cv:         &mut Canvas,
        terminal:   Terminal,
        font:       Arc<Font>,
        on_command: impl Fn(&str, &Terminal) + Clone + 'static,
        focus:      Shared<bool>,
    ) {
        let home = std::env::var("HOME").unwrap_or_default();
        Self::register_keys(cv, terminal.clone(), on_command, home.clone(), focus);
        Self::register_scroll(cv, terminal.clone());
        Self::register_update(cv, terminal, font, home);
    }

    fn register_keys(
        cv:         &mut Canvas,
        terminal:   Terminal,
        on_command: impl Fn(&str, &Terminal) + Clone + 'static,
        home:       String,
        focus:      Shared<bool>,
    ) {
        cv.on_key_press(move |cv, key| {
            // Only handle keys when the terminal panel has focus.
            if !{ *focus.get() } { return; }

            cv.set_var("_term_cursor_blink_t",  quartz::Value::from(0.0f32));
            cv.set_var("_term_cursor_blink_on", quartz::Value::from(true));

            if { terminal.state.get().running } {
                if let Key::Character(ch) = key {
                    if ch.chars().next() == Some('\x03') {
                        Self::kill(&terminal);
                        let lh = { terminal.settings.get().lh() };
                        let vh = Self::vh(cv, &terminal);
                        terminal.state.get_mut().snap_bottom(lh, vh);
                    }
                }
                return;
            }

            match key {
                Key::Character(ch) => {
                    let first = ch.chars().next().unwrap_or('\0');
                    if first.is_control() {
                        Self::handle_ctrl(cv, &terminal, first);
                        return;
                    }
                    let chars: Vec<char> = ch.chars().filter(|c| !c.is_control()).collect();
                    if chars.is_empty() { return; }
                    let start_col  = { terminal.state.get().cursor_col };
                    let start_byte = { let st = terminal.state.get(); char_to_byte(&st.input, start_col) };
                    let mut st = terminal.state.get_mut();
                    st.history_idx    = None;
                    st.tab_candidates = Vec::new();
                    st.tab_index      = 0;
                    let mut off = start_byte;
                    for c in &chars { st.input.insert(off, *c); off += c.len_utf8(); }
                    st.cursor_col = start_col + chars.len();
                }

                Key::Named(NamedKey::Tab) => {
                    let (input, cwd) = { let st = terminal.state.get(); (st.input.clone(), st.cwd.clone()) };
                    Self::handle_tab(&terminal, &input, &cwd, &home);
                }

                Key::Named(NamedKey::Enter) => {
                    let (input, cwd) = { let st = terminal.state.get(); (st.input.clone(), st.cwd.clone()) };
                    let cmd = input.trim().to_string();
                    let cwd_display = Self::display_cwd(&cwd, &home);
                    terminal.state.get_mut().push(Line::precolored(&[(&cwd_display, Some(COL_CWD))]));
                    terminal.state.get_mut().push(Line::precolored(&[
                        (PROMPT_IDLE, Some(COL_ECHO)),
                        (&input,      Some(COL_ECHO)),
                    ]));
                    {
                        let mut st    = terminal.state.get_mut();
                        st.input.clear();
                        st.cursor_col     = 0;
                        st.history_idx    = None;
                        st.tab_candidates = Vec::new();
                        st.tab_index      = 0;
                    }
                    let lh = { terminal.settings.get().lh() };
                    let vh = Self::vh(cv, &terminal);
                    terminal.state.get_mut().snap_bottom(lh, vh);
                    if !cmd.is_empty() {
                        on_command(&cmd, &terminal);
                        terminal.state.get_mut().snap_bottom(lh, vh);
                    }
                }

                Key::Named(NamedKey::Backspace) | Key::Named(NamedKey::Delete) => {
                    let (col, byte) = {
                        let st = terminal.state.get();
                        let b  = if st.cursor_col > 0 { char_to_byte(&st.input, st.cursor_col - 1) } else { usize::MAX };
                        (st.cursor_col, b)
                    };
                    if col > 0 {
                        let mut st = terminal.state.get_mut();
                        st.tab_candidates = Vec::new();
                        st.tab_index      = 0;
                        if byte < st.input.len() { st.input.remove(byte); st.cursor_col -= 1; }
                    }
                }

                Key::Named(NamedKey::ArrowLeft) => {
                    let col = { terminal.state.get().cursor_col };
                    if col > 0 { terminal.state.get_mut().cursor_col -= 1; }
                }

                Key::Named(NamedKey::ArrowRight) => {
                    let (col, max) = { let st = terminal.state.get(); (st.cursor_col, st.input.chars().count()) };
                    if col < max { terminal.state.get_mut().cursor_col += 1; }
                }

                Key::Named(NamedKey::ArrowUp) => {
                    let (empty, len) = { let st = terminal.state.get(); (st.history.is_empty(), st.history.len()) };
                    if empty { return; }
                    let cur       = { terminal.state.get().history_idx };
                    let idx       = cur.map(|i| i.saturating_sub(1)).unwrap_or(len - 1);
                    let new_input = { terminal.state.get().history[idx].clone() };
                    let mut st    = terminal.state.get_mut();
                    st.history_idx    = Some(idx);
                    st.input          = new_input;
                    st.cursor_col     = st.input.chars().count();
                    st.tab_candidates = Vec::new();
                    st.tab_index      = 0;
                }

                Key::Named(NamedKey::ArrowDown) => {
                    let (cur, hist_len) = { let st = terminal.state.get(); (st.history_idx, st.history.len()) };
                    match cur {
                        None => {}
                        Some(i) if i + 1 >= hist_len => {
                            let mut st    = terminal.state.get_mut();
                            st.history_idx    = None;
                            st.input.clear();
                            st.cursor_col     = 0;
                            st.tab_candidates = Vec::new();
                            st.tab_index      = 0;
                        }
                        Some(i) => {
                            let idx       = i + 1;
                            let new_input = { terminal.state.get().history[idx].clone() };
                            let mut st    = terminal.state.get_mut();
                            st.history_idx    = Some(idx);
                            st.input          = new_input;
                            st.cursor_col     = st.input.chars().count();
                            st.tab_candidates = Vec::new();
                            st.tab_index      = 0;
                        }
                    }
                }

                Key::Named(NamedKey::Home) => { terminal.state.get_mut().cursor_col = 0; }

                Key::Named(NamedKey::End) => {
                    let max = { terminal.state.get().input.chars().count() };
                    terminal.state.get_mut().cursor_col = max;
                }

                Key::Named(NamedKey::Space) => {
                    let (col, b) = { let st = terminal.state.get(); (st.cursor_col, char_to_byte(&st.input, st.cursor_col)) };
                    let mut st = terminal.state.get_mut();
                    st.tab_candidates = Vec::new();
                    st.tab_index      = 0;
                    st.input.insert(b, ' ');
                    st.cursor_col = col + 1;
                }

                _ => {}
            }
        });
    }

    fn handle_ctrl(cv: &mut Canvas, terminal: &Terminal, ch: char) {
        match ch {
            '\x03' => {
                Self::kill(terminal);
                let lh = { terminal.settings.get().lh() };
                let vh = Self::vh(cv, terminal);
                terminal.state.get_mut().snap_bottom(lh, vh);
            }
            '\x0c' => terminal.clear(),
            '\x01' => { terminal.state.get_mut().cursor_col = 0; }
            '\x05' => {
                let max = { terminal.state.get().input.chars().count() };
                terminal.state.get_mut().cursor_col = max;
            }
            '\x15' => {
                let col  = { terminal.state.get().cursor_col };
                let from = { let st = terminal.state.get(); char_to_byte(&st.input, col) };
                let mut st    = terminal.state.get_mut();
                st.input      = st.input[from..].to_string();
                st.cursor_col = 0;
            }
            '\x0b' => {
                let col = { terminal.state.get().cursor_col };
                let to  = { let st = terminal.state.get(); char_to_byte(&st.input, col) };
                terminal.state.get_mut().input.truncate(to);
            }
            _ => {}
        }
    }

    fn kill(terminal: &Terminal) {
        let mut st = terminal.state.get_mut();
        st.kill_child();
        st.push(Line::echo("^C".to_string()));
        st.input.clear();
        st.cursor_col  = 0;
        st.history_idx = None;
    }

    fn handle_tab(terminal: &Terminal, input: &str, cwd: &str, home: &str) {
        let word   = input.split_whitespace().last().unwrap_or("");
        let prefix = input[..input.len() - word.len()].to_string();
        let (scan_dir, partial, dir_prefix) = if let Some(slash) = word.rfind('/') {
            let dir_part = &word[..slash];
            let base     = word[slash + 1..].to_string();
            let resolved = if dir_part.starts_with('/') {
                dir_part.to_string()
            } else if dir_part.starts_with('~') {
                format!("{}{}", home, &dir_part[1..])
            } else {
                format!("{}/{}", cwd, dir_part)
            };
            (resolved, base, format!("{}/", dir_part))
        } else {
            (cwd.to_string(), word.to_string(), String::new())
        };

        let mut candidates: Vec<String> = std::fs::read_dir(&scan_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                  .map(|e| {
                      let name = e.file_name().to_string_lossy().into_owned();
                      if e.path().is_dir() { format!("{}/", name) } else { name }
                  })
                  .filter(|n| n.to_lowercase().starts_with(&partial.to_lowercase()))
                  .collect()
            })
            .unwrap_or_default();
        candidates.sort();

        if candidates.is_empty() { return; }

        let existing = { terminal.state.get().tab_candidates.clone() };

        if existing == candidates {
            let idx  = { terminal.state.get().tab_index };
            let next = (idx + 1) % candidates.len();
            let new_input = format!("{}{}{}", prefix, dir_prefix, candidates[next]);
            let new_col   = new_input.chars().count();
            let mut st    = terminal.state.get_mut();
            st.input      = new_input;
            st.cursor_col = new_col;
            st.tab_index  = next;
        } else if candidates.len() == 1 {
            let new_input = format!("{}{}{}", prefix, dir_prefix, candidates[0]);
            let new_col   = new_input.chars().count();
            let mut st    = terminal.state.get_mut();
            st.input          = new_input;
            st.cursor_col     = new_col;
            st.tab_candidates = Vec::new();
            st.tab_index      = 0;
        } else {
            let lcp       = Self::longest_common_prefix(&candidates);
            let new_input = format!("{}{}{}", prefix, dir_prefix, lcp);
            let new_col   = new_input.chars().count();
            terminal.state.get_mut().push(Line::output(candidates.join("  ")));
            let mut st    = terminal.state.get_mut();
            st.input          = new_input;
            st.cursor_col     = new_col;
            st.tab_candidates = candidates;
            st.tab_index      = 0;
        }
    }

    fn register_scroll(cv: &mut Canvas, terminal: Terminal) {
        cv.on_mouse_scroll(move |cv, (_dx, dy)| {
            let (ox, oy, vw, vh) = {
                let s = terminal.settings.get();
                let (cvw, cvh) = cv.canvas_size();
                (s.offset_x, s.offset_y, (cvw - s.offset_x).max(0.0), (cvh - s.offset_y).max(0.0))
            };
            if let Some((mx, my)) = cv.mouse_position() {
                if mx < ox || mx > ox + vw || my < oy || my > oy + vh { return; }
            } else { return; }
            if dy != 0.0 {
                let lh  = { terminal.settings.get().lh() };
                let cur = { terminal.state.get().scroll };
                terminal.state.get_mut().scroll = (cur + dy * SCROLL_LINES * lh).max(0.0);
            }
        });
    }

    fn register_update(cv: &mut Canvas, terminal: Terminal, font: Arc<Font>, home: String) {
        cv.on_update(move |cv| {
            let (cvw, cvh) = cv.canvas_size();
            if cvw < 1.0 || cvh < 1.0 { return; }

            let (ox, oy, vw, vh, lh, cw_char, pad_x, pad_y, fs, bg, col_text, col_error) = {
                let s  = terminal.settings.get();
                let vw = (cvw - s.offset_x).max(1.0);
                let vh = (cvh - s.offset_y).max(1.0);
                (s.offset_x, s.offset_y, vw, vh,
                 s.lh(), s.cw(), s.pad_x, s.pad_y, s.font_size,
                 s.bg, s.col_text, s.col_error)
            };
            let text_x = ox + pad_x;
            let text_w = vw - pad_x;
            let cols   = ((text_w / cw_char).floor() as usize).max(1);

            if let Some(o) = cv.get_game_object_mut("term_bg") {
                if o.position != (ox, oy) || o.size != (vw, vh) {
                    o.position = (ox, oy); o.size = (vw, vh);
                    o.set_image(tint_overlay(vw, vh, bg));
                }
            }

            if { terminal.state.get_mut().drain_stream() } {
                terminal.state.get_mut().scroll += lh;
            }

            let row_map = { let st = terminal.state.get(); Self::build_row_map(&st.buf, cols) };

            let (prompt_str, prompt_col, input_str, cwd) = {
                let st = terminal.state.get();
                let (ps, pc) = if st.running { (PROMPT_RUNNING, COL_PROMPT_RUNNING) }
                               else          { (PROMPT_IDLE,    COL_PROMPT_IDLE)    };
                (ps, pc, st.input.clone(), st.cwd.clone())
            };

            let cwd_display         = Self::display_cwd(&cwd, &home);
            let cwd_rows            = Self::wrap(&cwd_display, cols);
            let input_rows          = Self::wrap(&format!("{}{}", prompt_str, input_str), cols);
            let buf_visual_count    = row_map.len();
            let prompt_visual_start = buf_visual_count + cwd_rows.len();
            let total_visual        = prompt_visual_start + input_rows.len();

            {
                let hard_max = (total_visual as f32 * lh + vh * 0.1).max(0.0);
                let mut st   = terminal.state.get_mut();
                st.scroll    = st.scroll.max(0.0).min(hard_max);
            }

            let (scroll, cursor_col, running, dirty) = {
                let st = terminal.state.get();
                (st.scroll, st.cursor_col, st.running, st.dirty)
            };

            if { terminal.state.get().slot_count } == 0 {
                let mut i = 0usize;
                loop {
                    let name = format!("tl_{i}");
                    if cv.get_game_object(&name).is_none() { break; }
                    cv.remove_game_object(&name);
                    i += 1;
                }
            }

            let slots_needed  = ((vh / lh).ceil() as usize) + 3;
            let current_slots = { terminal.state.get().slot_count };
            if slots_needed > current_slots {
                for i in current_slots..slots_needed {
                    let name = format!("tl_{i}");
                    let mut o = GameObject::build(&name)
                        .position(text_x, oy - lh * 4.0).size(text_w, lh).layer(3)
                        .clip().clip_origin(text_x, oy).clip_size(text_w, vh).finish();
                    o.set_drawable(Box::new(make_plain_text(" ", Color(0,0,0,0), &font, fs, lh)));
                    cv.add_game_object(name, o);
                }
                terminal.state.get_mut().slot_count = slots_needed;
            }

            let slot_count = { terminal.state.get().slot_count };
            let first_row  = (scroll / lh).floor() as usize;
            let sub_offset = scroll - first_row as f32 * lh;

            for slot in 0..slot_count {
                let visual_idx = first_row + slot;
                let slot_y     = oy + pad_y + slot as f32 * lh - sub_offset;
                let name       = format!("tl_{slot}");

                if let Some(o) = cv.get_game_object_mut(&name) {
                    o.set_clip_origin(Some((text_x, oy)));
                    o.set_clip_size(Some((text_w, vh)));
                }

                if visual_idx >= total_visual {
                    if let Some(o) = cv.get_game_object_mut(&name) { o.position.1 = oy - lh * 4.0; }
                    continue;
                }

                let text_obj: Box<dyn Drawable> = if visual_idx < buf_visual_count {
                    Self::buf_line_drawable(&terminal, &row_map, visual_idx, cols, col_text, col_error, &font, fs, lh)
                } else if visual_idx < prompt_visual_start {
                    let row = cwd_rows.get(visual_idx - buf_visual_count).cloned().unwrap_or_default();
                    let txt = if row.is_empty() { " ".to_string() } else { row };
                    Box::new(make_plain_text(&txt, COL_CWD, &font, fs, lh))
                } else {
                    let row_idx    = visual_idx - prompt_visual_start;
                    let row        = input_rows.get(row_idx).cloned().unwrap_or_default();
                    let p_len      = prompt_str.chars().count();
                    if row_idx == 0 {
                        let input_part = row.chars().skip(p_len).collect::<String>();
                        let i_display  = if input_part.is_empty() { " ".to_string() } else { input_part };
                        Box::new(Text::new(vec![
                            Span::new(prompt_str.to_string(), fs, Some(lh), font.clone(), prompt_col, 0.0),
                            Span::new(i_display,              fs, Some(lh), font.clone(), col_text,   0.0),
                        ], None, Align::Left, None))
                    } else {
                        let txt = if row.is_empty() { " ".to_string() } else { row };
                        Box::new(make_plain_text(&txt, col_text, &font, fs, lh))
                    }
                };

                if let Some(o) = cv.get_game_object_mut(&name) {
                    o.position = (text_x, slot_y);
                    o.set_drawable(text_obj);
                }
            }

            if dirty { terminal.state.get_mut().dirty = false; }

            let p_len             = prompt_str.chars().count();
            let cursor_in_prompt  = p_len + cursor_col;
            let cursor_visual     = prompt_visual_start + cursor_in_prompt / cols.max(1);
            let cursor_col_in_row = cursor_in_prompt % cols.max(1);
            let cursor_slot_y     = oy + pad_y + (cursor_visual as f32 - first_row as f32) * lh - sub_offset;
            let cursor_x          = text_x + cursor_col_in_row as f32 * cw_char;
            let cursor_y          = cursor_slot_y + (lh - fs) * 0.5;
            let in_view           = cursor_slot_y >= oy - lh && cursor_slot_y < oy + vh
                                 && cursor_x >= ox && cursor_x < ox + vw;

            let bt: f32 = cv.get_f32("_term_cursor_blink_t") + 1.0 / 60.0;
            cv.set_var("_term_cursor_blink_t", quartz::Value::from(bt));
            if bt >= 0.53 {
                cv.set_var("_term_cursor_blink_t",  quartz::Value::from(0.0f32));
                let on = cv.get_bool("_term_cursor_blink_on");
                cv.set_var("_term_cursor_blink_on", quartz::Value::from(!on));
            }
            let blink_on = cv.get_bool("_term_cursor_blink_on");

            if let Some(o) = cv.get_game_object_mut("term_cursor") {
                o.size     = (cw_char, fs);
                o.position = (cursor_x, cursor_y);
                o.visible  = in_view && !running && blink_on;
                if in_view { o.set_image(tint_overlay(cw_char, fs, Color(255, 255, 255, 180))); }
            }
        });
    }

    fn vh(cv: &Canvas, terminal: &Terminal) -> f32 {
        let (_, ch) = cv.canvas_size();
        (ch - terminal.settings.get().offset_y).max(1.0)
    }

    fn display_cwd(cwd: &str, home: &str) -> String {
        if !home.is_empty() && cwd.starts_with(home) {
            format!("~{}", &cwd[home.len()..])
        } else {
            cwd.to_string()
        }
    }

    fn wrap(text: &str, cols: usize) -> Vec<String> {
        if cols == 0 { return vec![text.to_string()]; }
        let mut lines     = Vec::new();
        let mut remaining = text;
        loop {
            let n = remaining.chars().count();
            if n <= cols { lines.push(remaining.to_string()); break; }
            let mut brk   = cols;
            let chars: Vec<char> = remaining.chars().take(cols + 1).collect();
            for i in (0..cols).rev() {
                if chars[i].is_whitespace() { brk = i + 1; break; }
            }
            let end = remaining.char_indices().nth(brk).map(|(b,_)| b).unwrap_or(remaining.len());
            lines.push(remaining[..end].to_string());
            remaining = remaining[end..].trim_start_matches(' ');
            if remaining.is_empty() { break; }
        }
        if lines.is_empty() { lines.push(String::new()); }
        lines
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let (bytes, len) = (s.as_bytes(), s.len());
        let mut i = 0;
        while i < len {
            if bytes[i] == 0x1b && i + 1 < len && bytes[i+1] == b'[' {
                let mut j = i + 2;
                while j < len && !bytes[j].is_ascii_alphabetic() { j += 1; }
                i = j + 1;
            } else if bytes[i] == b'\r' {
                i += 1;
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        }
        out
    }

    fn row_count(line: &Line, cols: usize) -> usize {
        let plain = Self::strip_ansi(&line.text);
        let src   = if plain.trim().is_empty() { " ".to_string() } else { plain };
        Self::wrap(&src, cols).len().max(1)
    }

    fn build_row_map(lines: &[Line], cols: usize) -> Vec<(usize, usize)> {
        let mut entries = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            for r in 0..Self::row_count(line, cols) { entries.push((i, r)); }
        }
        entries
    }

    fn longest_common_prefix(items: &[String]) -> String {
        if items.is_empty() { return String::new(); }
        let first = &items[0];
        let len   = items[1..].iter().fold(first.chars().count(), |acc, item| {
            acc.min(first.chars().zip(item.chars()).take_while(|(a, b)| a == b).count())
        });
        first.chars().take(len).collect()
    }

    fn buf_line_drawable(
        terminal:   &Terminal,
        row_map:    &[(usize, usize)],
        visual_idx: usize,
        cols:       usize,
        col_text:   Color,
        col_error:  Color,
        font:       &Arc<Font>,
        fs:         f32,
        lh:         f32,
    ) -> Box<dyn Drawable> {
        let (buf_idx, sub_row) = row_map[visual_idx];
        let default_col = {
            let st = terminal.state.get();
            match st.buf[buf_idx].kind {
                LineKind::Err  => col_error,
                LineKind::Echo => COL_ECHO,
                _              => col_text,
            }
        };
        if sub_row == 0 {
            let d = { let mut st = terminal.state.get_mut(); build_line_text(&mut st.buf[buf_idx], default_col, font, fs, lh) };
            Box::new(d)
        } else {
            let row_text = {
                let st    = terminal.state.get();
                let plain = Self::strip_ansi(&st.buf[buf_idx].text);
                let src   = if plain.trim().is_empty() { " ".to_string() } else { plain };
                Self::wrap(&src, cols).into_iter().nth(sub_row).unwrap_or_default()
            };
            let txt = if row_text.is_empty() { " ".to_string() } else { row_text };
            Box::new(make_plain_text(&txt, default_col, font, fs, lh))
        }
    }
}

pub fn setup(cv: &mut Canvas, s: &TermSettings) {
    TerminalView::setup(cv, s);
}

pub fn register(
    cv:         &mut Canvas,
    terminal:   Terminal,
    font:       Arc<Font>,
    on_command: impl Fn(&str, &Terminal) + Clone + 'static,
    focus:      Shared<bool>,
) {
    TerminalView::register(cv, terminal, font, on_command, focus);
}