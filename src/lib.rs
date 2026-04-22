pub mod constants;
pub mod preferences;
pub mod objects;
pub mod logic;
pub mod terminal;
pub mod tabbar;

pub use terminal::Terminal;
pub use terminal::line::{Line, LineKind};
pub use logic::run_command::run_command;

use flowmango::LayerId;
use flowmango::Scene;
use quartz::{Arc, Color, Font, Shared};
use crate::preferences::TermSettings;
use crate::terminal::line::Line as TermLine;
use crate::terminal::state::State;
use quartz::Context;
use ramp::prism;

pub fn mount(
    _ctx:       &mut Context,
    scene:      &mut Scene,
    layer_id:   LayerId,
    font_bytes: Vec<u8>,
    settings:   Option<Shared<TermSettings>>,
    on_command: impl Fn(&str, &Terminal) + Clone + 'static,
) -> Terminal {
    let settings = settings.unwrap_or_else(|| Shared::new(TermSettings::default()));

    {
        let mut s    = settings.get_mut();
        s.col_cursor = Color(210, 210, 210, 255);
        s.col_prompt = Color(170, 170, 170, 255);
    }

    let font       = Arc::new(Font::from_bytes(&font_bytes).expect("terminal: bad font"));
    let scrollback = settings.get().scrollback;
    let state      = Shared::new(State::new(scrollback));
    let terminal   = Terminal { settings: settings.clone(), state: state.clone() };


    let cv = scene.get_layer_mut(layer_id).unwrap().canvas_mut();

    let raw_panel_y = settings.get().offset_y;
    cv.set_var("_term_panel_y", raw_panel_y);

    objects::tabbar_obj::setup(cv, &settings.get());
    logic::tabbar_obj::register(cv, settings.clone());
    {
        let mut s   = settings.get_mut();
        s.offset_y += tabbar::TAB_H;
    }

    // Terminal objects.
    objects::terminal_obj::setup(cv, &settings.get());
    logic::terminal_obj::register(
        cv, state.clone(), settings.clone(), font.clone(), on_command, terminal.clone(),
    );

    terminal
}