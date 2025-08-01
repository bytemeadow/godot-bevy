use bevy::{
    app::{App, Plugin},
    log::{
        Level, tracing,
        tracing_subscriber::{self, EnvFilter},
    },
};
use chrono::Local;
use godot::global::{godot_error, godot_print, godot_print_rich, godot_warn};
use std::{
    error::Error,
    path::{MAIN_SEPARATOR_STR, Path},
    string::ParseError,
};
use tracing_subscriber::{
    Layer, field::Visit, filter::FromEnvError, layer::SubscriberExt, util::SubscriberInitExt,
};

/// NOTE: This plugin is only available if the `godot_bevy_log` feature is enabled
pub struct GodotBevyLogPlugin {
    /// Filters logs using the [`EnvFilter`] format
    /// Behaves identically to Bevy's LogPlugin filter field, see https://docs.rs/bevy/latest/bevy/log/struct.LogPlugin.html
    pub filter: String,

    /// Filters out logs that are "less than" the given level.
    /// This can be further filtered using the `filter` setting.
    /// Behaves identically to Bevy's LogPlugin level field, see https://docs.rs/bevy/latest/bevy/log/struct.LogPlugin.html
    pub level: Level,

    /// Enable/disable color in output. NOTE: Enabling this incurs
    /// a performance penalty. Defaults to true.
    pub color: bool,

    /// Accepts timestamp formatting, see <https://docs.rs/chrono/0.4.41/chrono/format/strftime/index.html>
    /// You can disable the timestamp entirely by providing `None`.
    /// Example default format: `11:30:37.631`
    pub timestamp_format: Option<String>,
}

impl Default for GodotBevyLogPlugin {
    fn default() -> Self {
        Self {
            filter: bevy::log::DEFAULT_FILTER.to_string(),
            level: Level::INFO,
            color: true,
            // Timestamp formatting reference https://docs.rs/chrono/0.4.41/chrono/format/strftime/index.html
            timestamp_format: Some("%T%.3f".to_owned()),
        }
    }
}

impl Plugin for GodotBevyLogPlugin {
    fn build(&self, _app: &mut App) {
        // Copied behavior from https://docs.rs/bevy_log/0.16.1/src/bevy_log/lib.rs.html#279
        let default_filter = { format!("{},{}", self.level, self.filter) };
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|from_env_error| {
                _ = from_env_error
                    .source()
                    .and_then(|source| source.downcast_ref::<ParseError>())
                    .map(|parse_err| {
                        // we cannot use the `error!` macro here because the logger is not ready yet.
                        eprintln!(
                            "GodotBevyLogPlugin failed to parse filter from env: {parse_err}"
                        );
                    });

                Ok::<EnvFilter, FromEnvError>(EnvFilter::builder().parse_lossy(&default_filter))
            })
            .unwrap();

        let godot_proxy_layer = GodotProxyLayer {
            color: self.color,
            timestamp_format: self.timestamp_format.clone(),
        };

        #[cfg(feature = "trace_tracy")]
        tracing_subscriber::registry()
            .with(godot_proxy_layer)
            .with(filter_layer)
            .with(tracing_tracy::TracyLayer::default())
            .init();

        #[cfg(not(feature = "trace_tracy"))]
        tracing_subscriber::registry()
            .with(godot_proxy_layer)
            .with(filter_layer)
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
