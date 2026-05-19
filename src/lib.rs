mod constants;
mod ansi;
mod line;
mod state;
mod text;
mod terminal;
mod run_command;
mod components;

pub mod preferences;

pub use terminal::Terminal;
pub use line::{Line, LineKind};
pub use run_command::run_command;
pub use preferences::TermSettings;

use flowmango::{LayerId, Scene};
use quartz::{Arc, Font, Shared};
use ramp::prism::Context;
use state::State;

pub fn mount(
    _ctx:       &mut Context,
    scene:      &mut Scene,
    layer_id:   LayerId,
    font_bytes: Vec<u8>,
    settings:   Option<Shared<TermSettings>>,
    cwd:        String,
    on_command: impl Fn(&str, &Terminal) + Clone + 'static,
    focus:      Shared<bool>,
) -> Terminal {
    let settings = settings.unwrap_or_else(|| Shared::new(TermSettings::default()));

    {
        let mut s    = settings.get_mut();
        s.col_cursor = quartz::Color(210, 210, 210, 255);
        s.col_prompt = quartz::Color(170, 170, 170, 255);
    }

    let font       = Arc::new(Font::from_bytes(&font_bytes).expect("terminal: bad font"));
    let scrollback = settings.get().scrollback;
    let state      = Shared::new(State::new(scrollback, cwd));
    let terminal   = Terminal { settings: settings.clone(), state };

    let cv = scene.get_layer_mut(layer_id).unwrap().canvas_mut();

    components::terminal_view::setup(cv, &settings.get());
    components::terminal_view::register(cv, terminal.clone(), font, on_command, focus);

    terminal
}