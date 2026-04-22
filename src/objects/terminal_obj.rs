use quartz::tint_overlay;
use crate::preferences::TermSettings;
use flowmango::GameObject;
use quartz::Canvas;

pub fn setup(cv: &mut Canvas, s: &TermSettings) {
    cv.add_game_object("term_bg".into(),
        GameObject::build("term_bg")
            .position(s.offset_x, s.offset_y)
            .size(1.0, 1.0)
            .layer(0)
            .image(tint_overlay(1.0, 1.0, s.bg))
            .finish(),
    );

    cv.add_game_object("term_cursor".into(),
        GameObject::build("term_cursor")
            .position(-9999.0, -9999.0)
            .size(2.0, 14.0)
            .layer(6)
            .image(tint_overlay(2.0, 14.0, s.col_cursor))
            .finish(),
    );

    cv.set_var("_tb", quartz::Value::from(0.0f32));
    cv.set_var("_to", quartz::Value::from(true));
}