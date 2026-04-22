use quartz::{Canvas, Color, GameObject};
use quartz::tint_overlay;
use crate::preferences::TermSettings;
use crate::tabbar::{TAB_H, TAB_W, DIV_W, DIV_H_FRAC, TAB_COUNT};

pub const ICON_PATHS: [&str; TAB_COUNT] = [
    "resources/terminal.png",
    "resources/view.png",
    "resources/chat.png",
];

pub const ICON_PATHS_UNSELECTED: [&str; TAB_COUNT] = [
    "resources/unselected_terminal.png",
    "resources/unselected_view.png",
    "resources/unselected_chat.png",
];

pub const ICON_SIZE: f32 = crate::tabbar::ICON_SIZE;

pub fn setup(cv: &mut Canvas, _settings: &TermSettings) {
    // ── Background strip ──────────────────────────────────────────────────────
    {
        let obj = GameObject::build("tabbar_bg")
            .position(-9999.0, -9999.0)
            .size(1.0, TAB_H)
            .layer(8)
            .image(tint_overlay(1.0, TAB_H, Color(0, 0, 0, 1)))
            .finish();
        cv.add_game_object("tabbar_bg".into(), obj);
    }

    // ── Bottom separator ──────────────────────────────────────────────────────
    {
        let obj = GameObject::build("tabbar_sep")
            .position(-9999.0, -9999.0)
            .size(1.0, 1.0)
            .layer(9)
            .image(tint_overlay(1.0, 1.0, Color(45, 45, 45, 255)))
            .finish();
        cv.add_game_object("tabbar_sep".into(), obj);
    }

    for i in 0..TAB_COUNT {
        // ── Icon ─────────────────────────────────────────────────────────────
        let name        = tab_name(i);
        let placeholder = tint_overlay(ICON_SIZE, ICON_SIZE, Color(0, 0, 0, 0));
        let icon_obj    = GameObject::build(name)
            .position(-9999.0, -9999.0)
            .size(ICON_SIZE, ICON_SIZE)
            .layer(11)
            .image(placeholder)
            .finish();
        cv.add_game_object(name.into(), icon_obj);

        // ── Vertical divider between tab slots ────────────────────────────────
        let div_h    = TAB_H * DIV_H_FRAC;
        let div_name = format!("tabbar_div_{}", i);
        let div_obj  = GameObject::build(div_name.clone())
            .position(-9999.0, -9999.0)
            .size(DIV_W, div_h)
            .layer(9)
            .image(tint_overlay(DIV_W, div_h, Color(45, 45, 45, 255)))
            .finish();
        cv.add_game_object(div_name, div_obj);
    }

    // ── Chat panel ───────────────────────────────────────────────────────────
    {
        let mut obj = GameObject::build("tabbar_chat_msg")
            .position(-9999.0, -9999.0)
            .size(1.0, 1.0)
            .layer(10)
            .finish();
        obj.visible = false;
        cv.add_game_object("tabbar_chat_msg".into(), obj);
    }

    cv.set_var("tab_active",        0u8);
    cv.set_var("_tab_icons_loaded", false);
}

pub fn tab_name(i: usize) -> &'static str {
    match i {
        0 => "tab_term",
        1 => "tab_view",
        _ => "tab_chat",
    }
}

pub fn tab_accent_name(i: usize) -> String {
    format!("tab_accent_{}", i)
}