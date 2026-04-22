use quartz::Color;
use crate::constants::*;

#[derive(Clone)]
pub struct TermSettings {
    pub font_size:   f32,
    pub line_height: f32,
    pub pad_x:       f32,
    pub pad_y:       f32,
    pub offset_x:    f32,
    pub offset_y:    f32,
    pub bg:          Color,
    pub col_text:    Color,
    pub col_prompt:  Color,
    pub col_input:   Color,
    pub col_cursor:  Color,
    pub col_error:   Color,
    pub scrollback:  usize,
}

impl Default for TermSettings {
    fn default() -> Self {
        Self {
            font_size:   TERM_FONT_SIZE,
            line_height: TERM_LINE_HEIGHT,
            pad_x:       TERM_PAD_X,
            pad_y:       TERM_PAD_Y,
            offset_x:    0.0,
            offset_y:    0.0,
            bg:          Color(  0,   0,   0,   1),
            col_text:    Color(210, 210, 210, 255),
            col_prompt:  Color( 90, 180, 255, 255),
            col_input:   Color(255, 255, 255, 255),
            col_cursor:  Color( 90, 180, 255, 255),
            col_error:   Color(255,  90,  90, 255),
            scrollback:  TERM_SCROLLBACK,
        }
    }
}

impl TermSettings {
    pub fn lh(&self) -> f32 { self.font_size * self.line_height }
    pub fn cw(&self) -> f32 { self.font_size * 0.60 }
}