use std::fmt::Display;

use bindings::exports::wasi::logging::logging::Level;
use bindings::wasi::cli::stdout;
use bindings::wasi::clocks::system_clock::now;
use wit_bindgen::{block_on, spawn_local};

use crate::bindings::wasi::cli::stderr;
use crate::bindings::wit_stream;

mod bindings {
    wit_bindgen::generate!({
        world: "exports",
        generate_all,
    });
    use super::Component;
    export!(Component);
}

fn println<D: Display>(d: D, is_error: bool) {
    let data = format!("{}\n", d).as_bytes().to_vec();
    let (mut writer, reader) = wit_stream::new::<u8>();
    spawn_local(async move {
        writer.write_all(data).await;
        drop(writer);
    });
    if is_error {
        block_on(async move {
            stdout::write_via_stream(reader).await.unwrap();
        });
    } else {
        block_on(async move {
            stderr::write_via_stream(reader).await.unwrap();
        });
    }
}

struct Component;

fn level_to_str(level: Level) -> &'static str {
    match level {
        Level::Critical => "critical",
        Level::Error => "error",
        Level::Warn => "warning",
        Level::Info => "info",
        Level::Debug => "debug",
        Level::Trace => "trace",
    }
}

fn now_rfc3399() -> String {
    let timestamp = now();
    let duration = std::time::Duration::new(
        u64::try_from(timestamp.seconds).unwrap(),
        timestamp.nanoseconds,
    );
    let timestamp = std::time::UNIX_EPOCH + duration;
    let timestamp = chrono::DateTime::<chrono::Utc>::from(timestamp);
    timestamp.to_rfc3339()
}

impl bindings::exports::wasi::logging::logging::Guest for Component {
    fn log(level: Level, context: String, message: String) {
        let level_str = level_to_str(level);
        let message = format!("{}: {}", level_str, message);

        println(format!("{}: {} {}", context, now_rfc3399(), message), false);
    }
}
