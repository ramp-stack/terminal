use quartz::{Canvas, Color, Shared};
use quartz::{load_image_sized, tint_overlay};
use crate::preferences::TermSettings;
use crate::tabbar::{TAB_H, TAB_W, DIV_W, DIV_H_FRAC, TAB_COUNT, tab_x_rel};
use crate::objects::tabbar_obj::{tab_name, tab_accent_name, ICON_SIZE};
use crate::objects::tabbar_obj::ICON_PATHS_UNSELECTED;
use crate::objects::tabbar_obj::ICON_PATHS;

fn in_tab(mx: f32, my: f32, tab_x: f32, tab_y: f32) -> bool {
    mx >= tab_x && mx <= tab_x + TAB_W && my >= tab_y && my <= tab_y + TAB_H
}

fn remove_terminal_objects(cv: &mut Canvas) {
    cv.remove_game_object("term_bg");
    cv.remove_game_object("term_cursor");
    let mut idx = 0usize;
    loop {
        let slot = format!("tl_{}", idx);
        if cv.get_game_object(&slot).is_none() { break; }
        cv.remove_game_object(&slot);
        idx += 1;
    }
}

pub fn register(cv: &mut Canvas, settings: Shared<TermSettings>) {

    // ── Initialise canvas vars (prevent typed-getter panics on missing keys) ──
    if !cv.has_var("tab_active")         { cv.set_var("tab_active",         0u8);   }
    if !cv.has_var("_tab_icons_loaded")  { cv.set_var("_tab_icons_loaded",  false); }
    if !cv.has_var("_term_objects_exist"){ cv.set_var("_term_objects_exist", false); }
    if !cv.has_var("_term_panel_y")      { cv.set_var("_term_panel_y",       0.0f32); }

    // on_mouse_press
    {
        let settings = settings.clone();
        cv.on_mouse_press(move |cv, btn, (mx, my)| {
            use quartz::MouseButton;
            if btn != MouseButton::Left { return; }

            let ox    = settings.get().offset_x;
            let tab_y = cv.get_f32("_term_panel_y");

            let mut hit = None;
            for i in 0..TAB_COUNT {
                if in_tab(mx, my, ox + tab_x_rel(i), tab_y) {
                    hit = Some(i as u8);
                    break;
                }
            }
            let new_tab = match hit { Some(t) => t, None => return };
            if cv.get_u8("tab_active") == new_tab { return; }

            let old_tab = cv.get_u8("tab_active");
            cv.set_var("tab_active", new_tab);

            // Swap icon images
            for i in 0..TAB_COUNT {
                let is_active = i as u8 == new_tab;
                let path = if is_active { ICON_PATHS[i] } else { ICON_PATHS_UNSELECTED[i] };
                let icon = load_image_sized(path, ICON_SIZE, ICON_SIZE);
                if let Some(o) = cv.get_game_object_mut(tab_name(i)) {
                    o.set_image(icon);
                }
            }

            // Remove terminal objects when leaving tab 0
            if old_tab == 0 && new_tab != 0 {
                remove_terminal_objects(cv);
                cv.set_var("_term_objects_exist", false);
            }

            if let Some(o) = cv.get_game_object_mut("tabbar_chat_msg") {
                o.visible = new_tab == 2;
            }
        });
    }

    // on_update
    {
        let settings = settings.clone();

        cv.on_update(move |cv| {
            // Deferred icon load frame 1
            if !cv.get_bool("_tab_icons_loaded") {
                cv.set_var("_tab_icons_loaded", true);
                let active = cv.get_u8("tab_active");
                for i in 0..TAB_COUNT {
                    let path = if i as u8 == active { ICON_PATHS[i] } else { ICON_PATHS_UNSELECTED[i] };
                    let icon = load_image_sized(path, ICON_SIZE, ICON_SIZE);
                    if let Some(o) = cv.get_game_object_mut(tab_name(i)) {
                        o.set_image(icon);
                    }
                }
            }

            let ox      = settings.get().offset_x;
            let tab_y   = cv.get_f32("_term_panel_y");
            let (cw, _) = cv.canvas_size();
            let full_w  = (cw - ox).max(1.0);

            // Background
            if let Some(o) = cv.get_game_object_mut("tabbar_bg") {
                o.position = (ox, tab_y);
                if (o.size.0 - full_w).abs() > 0.5 {
                    o.size = (full_w, TAB_H);
                    o.set_image(tint_overlay(full_w, TAB_H, Color(10, 10, 10, 255)));
                }
            }

            // Separator
            if let Some(o) = cv.get_game_object_mut("tabbar_sep") {
                o.position = (ox, tab_y + TAB_H - 1.0);
                if (o.size.0 - full_w).abs() > 0.5 {
                    o.size = (full_w, 1.0);
                    o.set_image(tint_overlay(full_w, 1.0, Color(45, 45, 45, 255)));
                }
            }

            // Tab slots, accent bars, dividers
            let div_h         = TAB_H * DIV_H_FRAC;
            let div_y         = tab_y + (TAB_H - div_h) * 0.5;
            let icon_x_offset = (TAB_W - ICON_SIZE) * 0.5;
            let icon_y_offset = (TAB_H - ICON_SIZE) * 0.5;

            for i in 0..TAB_COUNT {
                let tx = ox + tab_x_rel(i);

                if let Some(o) = cv.get_game_object_mut(tab_name(i)) {
                    o.position = (tx + icon_x_offset, tab_y + icon_y_offset);
                    o.size     = (ICON_SIZE, ICON_SIZE);
                }

                if let Some(o) = cv.get_game_object_mut(&tab_accent_name(i)) {
                    o.position = (tx, tab_y + TAB_H - 2.0);
                    o.size     = (TAB_W, 2.0);
                }

                let div_name = format!("tabbar_div_{}", i);
                if let Some(o) = cv.get_game_object_mut(&div_name) {
                    o.position = (tx + TAB_W, div_y);
                    o.size     = (DIV_W, div_h);
                }
            }

            // Chat panel
            let (_, ch) = cv.canvas_size();
            let panel_h = (ch - tab_y - TAB_H).max(1.0);
            if let Some(o) = cv.get_game_object_mut("tabbar_chat_msg") {
                o.position = (ox, tab_y + TAB_H);
                o.size     = (full_w, panel_h);
            }
        });
    }
}