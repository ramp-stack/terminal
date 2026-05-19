use quartz::{Align, Arc, Color, Font, Span, Text};
use crate::ansi::StyledSpan;
use crate::line::Line;

pub(crate) fn build_line_text(
    line:    &mut Line,
    default: Color,
    font:    &Arc<Font>,
    fs:      f32,
    lh:      f32,
) -> Text {
    let spans: Vec<Span> = line.get_spans().iter().map(|s| {
        let color = s.color.unwrap_or(default);
        let txt   = if s.text.is_empty() { " ".to_string() } else { s.text.clone() };
        Span::new(txt, fs, Some(lh), font.clone(), color, 0.0)
    }).collect();
    Text::new(spans, None, Align::Left, None)
}

pub(crate) fn make_plain_text(
    text:  &str,
    color: Color,
    font:  &Arc<Font>,
    fs:    f32,
    lh:    f32,
) -> Text {
    let s = if text.is_empty() { " " } else { text };
    Text::new(
        vec![Span::new(s.to_string(), fs, Some(lh), font.clone(), color, 0.0)],
        None, Align::Left, None,
    )
}
