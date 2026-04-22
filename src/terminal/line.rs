use crate::terminal::ansi::{StyledSpan, parse_ansi};

#[derive(Clone)]
pub enum LineKind { Output, Stderr, Err, Echo }

#[derive(Clone)]
pub struct Line {
    pub text:    String,
    pub kind:    LineKind,
    pub(crate) spans_cache: Option<Vec<StyledSpan>>,
}

impl Line {
    pub fn output(s: impl Into<String>) -> Self { Self { text: s.into(), kind: LineKind::Output, spans_cache: None } }
    pub fn stderr(s: impl Into<String>) -> Self { Self { text: s.into(), kind: LineKind::Stderr, spans_cache: None } }
    pub fn error(s:  impl Into<String>) -> Self { Self { text: s.into(), kind: LineKind::Err,    spans_cache: None } }
    pub(crate) fn echo(s: impl Into<String>) -> Self { Self { text: s.into(), kind: LineKind::Echo, spans_cache: None } }

    /// Build a line with pre-colored segments: &[("text", Option<Color>)].
    /// Callers never need to touch StyledSpan directly.
    pub fn precolored(segments: &[(&str, Option<quartz::Color>)]) -> Self {
        let spans = segments.iter().map(|(t, c)| StyledSpan {
            text:  t.to_string(),
            color: *c,
        }).collect();
        Self { text: String::new(), kind: LineKind::Output, spans_cache: Some(spans) }
    }

    pub(crate) fn get_spans(&mut self) -> &[StyledSpan] {
        if self.spans_cache.is_none() {
            self.spans_cache = Some(match self.kind {
                LineKind::Output | LineKind::Stderr => parse_ansi(&self.text),
                _ => vec![StyledSpan { text: self.text.clone(), color: None }],
            });
        }
        self.spans_cache.as_ref().unwrap()
    }
}