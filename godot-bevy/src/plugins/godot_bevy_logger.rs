use bevy::{
    app::{App, Plugin},
    log::{
        Level, tracing,
        tracing_subscriber::{self, EnvFilter},
    },
};
use chrono::Local;
use godot::global::{godot_error, godot_print, godot_print_rich, godot_warn};
use std::path::{MAIN_SEPARATOR_STR, Path};
use tracing_subscriber::{
    Layer, field::Visit, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

pub struct GodotBevyLogPlugin {
    /// Logs messages of this level or higher severity. Defaults to `LevelFilter::INFO`
    level_filter: LevelFilter,

    /// Enable/disable color in output. NOTE: Enabling this incurs
    /// a performance penalty. Defaults to true.
    color: bool,

    /// Accepts timestamp formatting, see <https://docs.rs/chrono/0.4.41/chrono/format/strftime/index.html>
    /// You can disable the timestamp entirely by providing `None`.
    /// Example default format: `11:30:37.631`
    timestamp_format: Option<String>,
}

impl Default for GodotBevyLogPlugin {
    fn default() -> Self {
        Self {
            level_filter: LevelFilter::INFO,

            color: true,

            // Timestamp formatting reference https://docs.rs/chrono/0.4.41/chrono/format/strftime/index.html
            timestamp_format: Some("%T%.3f".to_owned()),
        }
    }
}

impl Plugin for GodotBevyLogPlugin {
    fn build(&self, _app: &mut App) {
        let env_filter = EnvFilter::builder()
            .with_default_directive(self.level_filter.into())
            // Add override support via RUST_LOG env variable, e.g., `RUST_LOG=WARN cargo run` will filter-in warning and higher messages only
            .from_env_lossy();

        tracing_subscriber::registry()
            .with(GodotProxyLayer {
                color: self.color,
                timestamp_format: self.timestamp_format.clone(),
            })
            .with(env_filter)
            .init();
    }
}

struct GodotProxyLayerVisitor(Option<String>);

impl Visit for GodotProxyLayerVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = Some(format!("{value:?}"))
        }
    }
}

struct GodotProxyLayer {
    color: bool,
    timestamp_format: Option<String>,
}

impl<S> Layer<S> for GodotProxyLayer
where
    S: tracing::Subscriber,
{
    // When choosing colors in here, I tried to pick colors that were (a) gentler on the eyes when
    // using the default godot theme, and (b) which provided the highest contrast for user
    // generated content (actual message, level) and lower contrast for content that is generated
    // (timestamp, location). The ultimate goal was to optimize for fast readability against
    // dark themes (godot default and typical terminals)
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let mut msg_vistor = GodotProxyLayerVisitor(None);
        event.record(&mut msg_vistor);

        // Timestamp formatting reference https://docs.rs/chrono/0.4.41/chrono/format/strftime/index.html
        let timestamp = if let Some(format) = &self.timestamp_format {
            format!("{} ", Local::now().format(format))
        } else {
            "".to_string()
        };

        let level = match self.color {
            true => match *metadata.level() {
                Level::TRACE => "[color=LightGreen]T[/color]",
                Level::DEBUG => "[color=LightGreen]D[/color]",
                Level::INFO => "[color=LightGreen]I[/color]",
                Level::WARN => "[color=Yellow]W[/color]",
                Level::ERROR => "[color=Salmon]E[/color]",
            },

            false => match *metadata.level() {
                Level::TRACE => "T",
                Level::DEBUG => "D",
                Level::INFO => "I",
                Level::WARN => "W",
                Level::ERROR => "E",
            },
        };

        let msg = msg_vistor.0.unwrap_or_default();

        let short_location = if let Some(file) = metadata.file() {
            let path = Path::new(file);

            let mut x = path.iter().rev().take(2);
            let file = x.next().unwrap_or_default().to_string_lossy();
            let parent = if let Some(parent) = x.next() {
                format!("{}{}", parent.to_string_lossy(), MAIN_SEPARATOR_STR)
            } else {
                String::new()
            };

            format!("{}{}:{}", parent, file, metadata.line().unwrap_or_default())
        } else {
            String::new()
        };

        match self.color {
            true => godot_print_rich!(
                "[color=DimGray]{}[/color]{} {} [color=DimGray]@ {}[/color]",
                timestamp,
                level,
                msg,
                short_location
            ),

            false => godot_print!("{}{} {} @ {}", timestamp, level, msg, short_location),
        };

        match *metadata.level() {
            Level::WARN => {
                godot_warn!("{}", msg);
            }
            Level::ERROR => {
                godot_error!("{}", msg);
            }
            _ => {}
        };
    }
}
