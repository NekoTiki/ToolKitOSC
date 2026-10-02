//! Console logging styled after Vite's dev-server output — local time, a bracketed tag, and a
//! scope, e.g. `19:54:24 [vite] (client) hmr update /src/assets/main.css`. Built on `tracing`,
//! the standard structured-logging crate for the Rust ecosystem, with a custom formatter that
//! reproduces that look instead of tracing-subscriber's default `2024-01-01T00:00:00Z INFO ...`.
use std::fmt;

use chrono::Local;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// Stands in for Vite's `[vite]` tag — the one constant fragment of every line.
const TAG: &str = "vrc-osc";

/// Sets up the global `tracing` subscriber. Call once, as early as possible in `run()`.
///
/// Verbosity defaults to `info` and everything above; override with the `RUST_LOG` env var
/// (same syntax `tracing-subscriber`/`env_logger` use, e.g. `RUST_LOG=debug` or
/// `RUST_LOG=toolkitosc_lib::osc=trace`).
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(filter).event_format(ViteStyle).init();
}

struct ViteStyle;

impl<S, N> FormatEvent<S, N> for ViteStyle
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>
    ) -> fmt::Result {
        let ansi = writer.has_ansi_escapes();
        let now = Local::now().format("%H:%M:%S");
        let level = *event.metadata().level();
        let scope = scope_of(event.metadata().target());

        if ansi {
            write!(writer, "\x1b[2m{now}\x1b[0m \x1b[36m[{TAG}]\x1b[0m \x1b[2m({scope})\x1b[0m ")?;
        } else {
            write!(writer, "{now} [{TAG}] ({scope}) ")?;
        }

        match level {
            Level::ERROR if ansi => write!(writer, "\x1b[31merror\x1b[0m ")?,
            Level::ERROR => write!(writer, "error ")?,
            Level::WARN if ansi => write!(writer, "\x1b[33mwarn\x1b[0m ")?,
            Level::WARN => write!(writer, "warn ")?,
            _ => {}
        }

        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// Groups a `tracing` target (a full module path, e.g. `toolkitosc_lib::osc::udp`) down to
/// its first module segment (`osc`) — the equivalent of Vite's `(client)`/`(server)` scope tag.
fn scope_of(target: &str) -> &str {
    let rest = target.strip_prefix("toolkitosc_lib").unwrap_or(target);
    let rest = rest.strip_prefix("::").unwrap_or(rest);
    let first = rest.split("::").next().unwrap_or("");

    if first.is_empty() { "app" } else { first }
}
