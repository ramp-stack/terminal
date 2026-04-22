use quartz::{Arc, Color, Font, Shared, tint_overlay, NamedKey};
use crate::preferences::TermSettings;
use crate::terminal::Terminal;
use crate::terminal::line::{Line, LineKind};
use crate::terminal::state::{State, char_to_byte};
use crate::terminal::text::{build_line_text, make_plain_text};
use quartz::Canvas;
use quartz::GameObject;
use quartz::Key;

pub fn register(
    cv:         &mut Canvas,
    state:      Shared<State>,
    settings:   Shared<TermSettings>,
    font:       Arc<Font>,
    on_command: impl Fn(&str, &Terminal) + Clone + 'static,
    terminal:   Terminal,
) {
    // ── Initialise canvas vars (prevent get_bool/get_f32 panics on missing keys) ──
    if !cv.has_var("_term_objects_exist") { cv.set_var("_term_objects_exist", false); }
    if !cv.has_var("_to")                { cv.set_var("_to", true); }
    if !cv.has_var("_tb")               { cv.set_var("_tb", 0.0f32); }
    if !cv.has_var("tab_active")        { cv.set_var("tab_active", 0u8); }

    // ── Key handler ───────────────────────────────────────────────────────────
    let state_k    = state.clone();
    let settings_k = settings.clone();
    let term_k     = terminal.clone();
    let on_cmd_k   = on_command.clone();

    cv.on_key_press(move |cv, key| {
        // Ignore keypresses when not on terminal tab
        if cv.get_u8("tab_active") != 0 { return; }

        let (ox, oy, vw, vh) = {
            let s = settings_k.get();
            let (cvw, cvh) = cv.canvas_size();
            (s.offset_x, s.offset_y, (cvw - s.offset_x).max(0.0), (cvh - s.offset_y).max(0.0))
        };
        if let Some((mx, my)) = cv.mouse_position() {
            if mx < ox || mx > ox + vw || my < oy || my > oy + vh { return; }
        } else { return; }

        cv.set_var("_tb", quartz::Value::from(0.0f32));
        cv.set_var("_to", quartz::Value::from(true));

        let ctrl = cv.is_key_held(&Key::Named(NamedKey::Control));

        // ── Ctrl combos ───────────────────────────────────────────────────────
        if ctrl {
            if let Key::Character(ch) = key {
                match ch.to_lowercase().as_str() {
                    "c" => {
                        let mut st = state_k.get_mut();
                        st.kill_child();
                        st.push(Line::echo("^C".to_string()));
                        st.input.clear();
                        st.cursor_col  = 0;
                        st.h_scroll    = 0.0;
                        st.history_idx = None;
                        let s = settings_k.get();
                        let (_, ch) = cv.canvas_size();
                        let vh2 = (ch - s.offset_y).max(1.0);
                        st.snap_bottom(s.lh(), vh2);
                    }
                    "l" => { term_k.clear(); }
                    "a" => { state_k.get_mut().cursor_col = 0; }
                    "e" => {
                        let max = state_k.get().input.chars().count();
                        state_k.get_mut().cursor_col = max;
                    }
                    "u" => {
                        let mut st = state_k.get_mut();
                        let col       = st.cursor_col;
                        let keep_from = char_to_byte(&st.input, col);
                        st.input      = st.input[keep_from..].to_string();
                        st.cursor_col = 0;
                    }
                    "k" => {
                        let mut st  = state_k.get_mut();
                        let col     = st.cursor_col;
                        let keep_to = char_to_byte(&st.input, col);
                        st.input.truncate(keep_to);
                    }
                    _ => {}
                }
            }
            return;
        }

        if state_k.get().running { return; }

        // ── Bare keys ─────────────────────────────────────────────────────────
        match key {
            Key::Named(NamedKey::ArrowUp) => {
                let mut st = state_k.get_mut();
                if st.history.is_empty() { return; }
                let new_idx = match st.history_idx {
                    None    => st.history.len() - 1,
                    Some(i) => i.saturating_sub(1),
                };
                st.history_idx = Some(new_idx);
                st.input       = st.history[new_idx].clone();
                st.cursor_col  = st.input.chars().count();
            }
            Key::Named(NamedKey::ArrowDown) => {
                let mut st = state_k.get_mut();
                match st.history_idx {
                    None => {}
                    Some(i) if i + 1 >= st.history.len() => {
                        st.history_idx = None;
                        st.input.clear();
                        st.cursor_col = 0;
                    }
                    Some(i) => {
                        let new_idx    = i + 1;
                        st.history_idx = Some(new_idx);
                        st.input       = st.history[new_idx].clone();
                        st.cursor_col  = st.input.chars().count();
                    }
                }
            }
            Key::Named(NamedKey::Enter) => {
                let cmd = {
                    let mut st = state_k.get_mut();
                    let cmd = st.input.trim().to_string();
                    if !cmd.is_empty() { st.push(Line::echo(format!("> {cmd}"))); }
                    st.input.clear();
                    st.cursor_col  = 0;
                    st.h_scroll    = 0.0;
                    st.history_idx = None;
                    let s = settings_k.get();
                    let (_, ch) = cv.canvas_size();
                    let vh2 = (ch - s.offset_y).max(1.0);
                    st.snap_bottom(s.lh(), vh2);
                    cmd
                };
                if !cmd.is_empty() {
                    on_cmd_k(&cmd, &term_k);
                    let s = settings_k.get();
                    let (_, ch) = cv.canvas_size();
                    let vh2 = (ch - s.offset_y).max(1.0);
                    state_k.get_mut().snap_bottom(s.lh(), vh2);
                }
            }
            Key::Named(NamedKey::Backspace) | Key::Named(NamedKey::Delete) => {
                let mut st = state_k.get_mut();
                if st.cursor_col > 0 {
                    let b = char_to_byte(&st.input, st.cursor_col - 1);
                    if b < st.input.len() { st.input.remove(b); st.cursor_col -= 1; }
                }
            }
            Key::Named(NamedKey::ArrowLeft) => {
                let mut st = state_k.get_mut();
                if st.cursor_col > 0 { st.cursor_col -= 1; }
            }
            Key::Named(NamedKey::ArrowRight) => {
                let mut st  = state_k.get_mut();
                let max     = st.input.chars().count();
                if st.cursor_col < max { st.cursor_col += 1; }
            }
            Key::Named(NamedKey::Home) => { state_k.get_mut().cursor_col = 0; }
            Key::Named(NamedKey::End)  => {
                let max = state_k.get().input.chars().count();
                state_k.get_mut().cursor_col = max;
            }
            Key::Named(NamedKey::Space) => {
                let mut st = state_k.get_mut();
                let b = char_to_byte(&st.input, st.cursor_col);
                st.input.insert(b, ' '); st.cursor_col += 1;
            }
            Key::Character(ch) => {
                let mut st = state_k.get_mut();
                st.history_idx = None;
                for c in ch.chars() {
                    if c.is_control() { continue; }
                    let b = char_to_byte(&st.input, st.cursor_col);
                    st.input.insert(b, c); st.cursor_col += 1;
                }
            }
            _ => {}
        }
    });

    // ── Scroll ────────────────────────────────────────────────────────────────
    let state_s    = state.clone();
    let settings_s = settings.clone();

    cv.on_mouse_scroll(move |cv, (dx, dy)| {
        if cv.get_u8("tab_active") != 0 { return; }

        let (ox, oy, vw, vh, fs) = {
            let s = settings_s.get();
            let (cvw, cvh) = cv.canvas_size();
            (s.offset_x, s.offset_y, (cvw - s.offset_x).max(0.0), (cvh - s.offset_y).max(0.0), s.font_size)
        };
        if let Some((mx, my)) = cv.mouse_position() {
            if mx < ox || mx > ox + vw || my < oy || my > oy + vh { return; }
        } else { return; }

        let speed   = fs * 0.8;
        let max_vel = speed * 6.0;

        if dy != 0.0 {
            let dir  = if dy > 0.0 { 1.0f32 } else { -1.0 };
            let push = dy.abs() * speed;
            let cur  = state_s.get().scroll_vel;
            state_s.get_mut().scroll_vel = if cur == 0.0 || cur.signum() == dir {
                (cur + dir * push).clamp(-max_vel, max_vel)
            } else { dir * push };
        }
        if dx != 0.0 {
            let dir  = if dx > 0.0 { 1.0f32 } else { -1.0 };
            let push = dx.abs() * speed;
            let cur  = state_s.get().h_scroll_vel;
            state_s.get_mut().h_scroll_vel = if cur == 0.0 || cur.signum() == dir {
                (cur + dir * push).clamp(-max_vel, max_vel)
            } else { dir * push };
        }
    });

    // ── Update ────────────────────────────────────────────────────────────────
    let state_u    = state.clone();
    let settings_u = settings.clone();
    let font_u     = font.clone();

    cv.on_update(move |cv| {
        let (cvw, cvh) = cv.canvas_size();
        if cvw < 1.0 || cvh < 1.0 { return; }

        // ── Tab guard — drain stream but skip all rendering ───────────────────
        if cv.get_u8("tab_active") != 0 {
            state_u.get_mut().drain_stream();
            return;
        }

        // ── Recreate GameObjects if removed while on another tab ──────────────
        // State (buf, scroll, history, running) is untouched — session resumes.
        if !cv.get_bool("_term_objects_exist") {
            cv.set_var("_term_objects_exist", true);

            let s = settings_u.get();
            let bg_obj = GameObject::build("term_bg")
                .position(s.offset_x, s.offset_y)
                .size(1.0, 1.0)
                .layer(0)
                .image(tint_overlay(1.0, 1.0, s.bg))
                .finish();
            cv.add_game_object("term_bg".into(), bg_obj);

            let cursor_obj = GameObject::build("term_cursor")
                .position(-9999.0, -9999.0)
                .size(2.0, s.font_size)
                .layer(6)
                .image(tint_overlay(2.0, s.font_size, s.col_cursor))
                .finish();
            cv.add_game_object("term_cursor".into(), cursor_obj);

            // Reset slot count so on_update respawns all tl_N objects
            // from the existing buffer this tick
            state_u.get_mut().slot_count = 0;
            state_u.get_mut().dirty      = true;
        }

        let (ox, oy, vw, vh, lh, cw, pad_x, pad_y, fs,
             bg, col_cursor, col_prompt, col_text, col_error) = {
            let s = settings_u.get();
            let vw = (cvw - s.offset_x).max(1.0);
            let vh = (cvh - s.offset_y).max(1.0);
            (s.offset_x, s.offset_y, vw, vh,
             s.lh(), s.cw(), s.pad_x, s.pad_y, s.font_size,
             s.bg, s.col_cursor, s.col_prompt, s.col_text, s.col_error)
        };

        if let Some(o) = cv.get_game_object_mut("term_bg") {
            if o.position != (ox, oy) || o.size != (vw, vh) {
                o.position = (ox, oy); o.size = (vw, vh);
                o.set_image(tint_overlay(vw, vh, bg));
            }
        }

        {
            let changed = state_u.get_mut().drain_stream();
            if changed { state_u.get_mut().snap_bottom(lh, vh); }
        }

        {
            let vel = state_u.get().scroll_vel;
            if vel.abs() > 0.3 {
                let mut st = state_u.get_mut();
                st.scroll    += vel;
                st.scroll_vel = vel * 0.88;
                st.clamp_v(lh, vh);
            } else {
                let mut st = state_u.get_mut();
                st.scroll_vel = 0.0;
                st.clamp_v(lh, vh);
            }
        }

        {
            let vel = state_u.get().h_scroll_vel;
            if vel.abs() > 0.3 {
                let mut st = state_u.get_mut();
                st.h_scroll     = (st.h_scroll + vel).max(0.0);
                st.h_scroll_vel = vel * 0.88;
            } else {
                let mut st = state_u.get_mut();
                st.h_scroll_vel = 0.0;
                st.h_scroll     = st.h_scroll.max(0.0);
            }
        }

        let (scroll, h_scroll, buf_len, cursor_col, running, dirty) = {
            let st = state_u.get();
            (st.scroll, st.h_scroll, st.buf.len(), st.cursor_col, st.running, st.dirty)
        };

        let slots_needed  = ((vh / lh).ceil() as usize) + 3;
        let current_slots = state_u.get().slot_count;
        if slots_needed > current_slots {
            for i in current_slots..slots_needed {
                let name = format!("tl_{i}");
                let mut o = GameObject::build(&name)
                    .position(ox + pad_x, oy - lh * 4.0)
                    .size(4000.0, lh)
                    .layer(3)
                    .clip()
                    .clip_origin(ox + pad_x, oy)
                    .clip_size(vw - pad_x + h_scroll, vh)
                    .finish();
                o.set_drawable(Box::new(make_plain_text(" ", Color(0,0,0,0), &font_u, fs, lh)));
                cv.add_game_object(name, o);
            }
            state_u.get_mut().slot_count = slots_needed;
        }
        let slot_count = state_u.get().slot_count;

        let first_line = (scroll / lh).floor() as usize;
        let sub_offset = scroll - first_line as f32 * lh;
        let total      = buf_len + 1;

        for slot in 0..slot_count {
            let logical = first_line + slot;
            let slot_y  = oy + pad_y + slot as f32 * lh - sub_offset;
            let slot_x  = ox + pad_x - h_scroll;
            let name    = format!("tl_{slot}");

            if let Some(o) = cv.get_game_object_mut(&name) {
                o.set_clip_origin(Some((ox + pad_x, oy)));
                o.set_clip_size(Some((vw - pad_x + h_scroll, vh)));
            }

            if logical >= total {
                if let Some(o) = cv.get_game_object_mut(&name) { o.position.1 = oy - lh * 4.0; }
                continue;
            }

            if logical < buf_len {
                let text = {
                    let mut st = state_u.get_mut();
                    let line = &mut st.buf[logical];
                    let default_col = match line.kind {
                        LineKind::Err  => col_error,
                        LineKind::Echo => col_prompt,
                        _              => col_text,
                    };
                    build_line_text(line, default_col, &font_u, fs, lh)
                };
                if let Some(o) = cv.get_game_object_mut(&name) {
                    o.position = (slot_x, slot_y);
                    o.set_drawable(Box::new(text));
                }
            } else {
                let (input_text, color) = {
                    let st  = state_u.get();
                    let pre = if running { "… " } else { "> " };
                    (format!("{}{}", pre, st.input), col_prompt)
                };
                if let Some(o) = cv.get_game_object_mut(&name) {
                    o.position = (slot_x, slot_y);
                    let t = make_plain_text(&input_text, color, &font_u, fs, lh);
                    o.set_drawable(Box::new(t));
                }
            }
        }

        if dirty { state_u.get_mut().dirty = false; }

        let input_logical = buf_len;
        let input_slot_y  = oy + pad_y + (input_logical as f32 - first_line as f32) * lh - sub_offset;
        let prompt_w      = cw * 2.0;
        let cursor_x      = ox + pad_x - h_scroll + prompt_w + cursor_col as f32 * cw;
        let cursor_y      = input_slot_y + (lh - fs) * 0.5;

        let in_view = input_slot_y >= oy
                   && input_slot_y + lh <= oy + vh
                   && cursor_x    >= ox
                   && cursor_x     < ox + vw;

        let bt: f32 = cv.get_f32("_tb") + 1.0 / 60.0;
        cv.set_var("_tb", quartz::Value::from(bt));
        if bt >= 0.53 {
            cv.set_var("_tb", quartz::Value::from(0.0f32));
            let on = cv.get_bool("_to");
            cv.set_var("_to", quartz::Value::from(!on));
        }
        let cursor_on = cv.get_bool("_to") && !running;

        let cursor_w = 2.0f32;
        let cursor_h = fs;
        if let Some(o) = cv.get_game_object_mut("term_cursor") {
            o.size     = (cursor_w, cursor_h);
            o.position = (cursor_x, cursor_y);
            o.visible  = in_view && cursor_on;
            if in_view { o.set_image(tint_overlay(cursor_w, cursor_h, col_cursor)); }
        }
    });
}