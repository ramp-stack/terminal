use quartz::Color;

#[derive(Clone)]
pub struct StyledSpan {
    pub text:  String,
    pub color: Option<Color>,
}

pub fn parse_ansi(s: &str) -> Vec<StyledSpan> {
    const ANSI16: [Color; 16] = [
        Color( 30,  30,  30, 255), Color(205,  49,  49, 255),
        Color( 13, 188, 121, 255), Color(229, 229,  16, 255),
        Color( 36, 114, 200, 255), Color(188,  63, 188, 255),
        Color( 17, 168, 205, 255), Color(229, 229, 229, 255),
        Color(102, 102, 102, 255), Color(241,  76,  76, 255),
        Color( 35, 209, 139, 255), Color(245, 245,  67, 255),
        Color( 59, 142, 234, 255), Color(214, 112, 214, 255),
        Color( 41, 184, 219, 255), Color(229, 229, 229, 255),
    ];

    fn ansi256_color(n: u8) -> Color {
        if n < 16 { return ANSI16[n as usize]; }
        if n >= 232 {
            let v = (8 + (n - 232) as u16 * 10).min(255) as u8;
            return Color(v, v, v, 255);
        }
        let i = n - 16;
        let r = i / 36; let g = (i % 36) / 6; let b = i % 6;
        let c = |v: u8| if v == 0 { 0u8 } else { 55 + 40 * v };
        Color(c(r), c(g), c(b), 255)
    }

    let mut spans: Vec<StyledSpan> = Vec::new();
    let mut cur_color: Option<Color> = None;
    let mut run = String::new();
    let bytes = s.as_bytes();
    let len   = bytes.len();
    let mut i = 0;

    macro_rules! flush { () => {
        if !run.is_empty() {
            spans.push(StyledSpan { text: run.clone(), color: cur_color });
            run.clear();
        }
    }; }

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            let seq_start = i + 2;
            let mut j = seq_start;
            while j < len && !bytes[j].is_ascii_alphabetic() { j += 1; }
            let terminator = if j < len { bytes[j] } else { 0 };
            let params_str = std::str::from_utf8(&bytes[seq_start..j]).unwrap_or("");
            i = j + 1;
            if terminator != b'm' { continue; }
            let params: Vec<u32> = if params_str.is_empty() { vec![0] }
                else { params_str.split(';').map(|p| p.parse().unwrap_or(0)).collect() };
            flush!();
            let mut pi = 0;
            while pi < params.len() {
                match params[pi] {
                    0           => { cur_color = None; }
                    1..=9       => {}
                    n @ 30..=37 => { cur_color = Some(ANSI16[(n - 30) as usize]); }
                    39          => { cur_color = None; }
                    n @ 90..=97 => { cur_color = Some(ANSI16[(n - 90 + 8) as usize]); }
                    38 => {
                        if params.get(pi+1) == Some(&5) && pi+2 < params.len() {
                            cur_color = Some(ansi256_color(params[pi+2] as u8)); pi += 2;
                        } else if params.get(pi+1) == Some(&2) && pi+4 < params.len() {
                            cur_color = Some(Color(
                                params[pi+2] as u8, params[pi+3] as u8, params[pi+4] as u8, 255,
                            )); pi += 4;
                        }
                    }
                    _ => {}
                }
                pi += 1;
            }
        } else if bytes[i] == b'\r' {
            i += 1;
        } else {
            run.push(bytes[i] as char);
            i += 1;
        }
    }
    flush!();
    if spans.is_empty() { spans.push(StyledSpan { text: " ".into(), color: None }); }
    spans
}