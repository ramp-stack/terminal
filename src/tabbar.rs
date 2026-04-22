pub const TAB_H:      f32 = 40.0;
pub const TAB_W:      f32 = 48.0;
pub const ICON_SIZE:  f32 = 20.0;
pub const DIV_W:      f32 = 1.0;
pub const DIV_H_FRAC: f32 = 0.5;
pub const TAB_STRIDE: f32 = TAB_W + DIV_W;
pub const TAB_COUNT:  usize = 3;

#[inline]
pub fn tab_x_rel(i: usize) -> f32 {
    i as f32 * TAB_STRIDE
}