use std::fmt::Display;

use bindings::exports::wasi::logging::logging::Level;
use bindings::wasi::cli::stdout;
use bindings::wasi::clocks::system_clock::now;
use wit_bindgen::spawn_local;

use crate::bindings::wasi::cli::stderr;
use crate::bindings::wit_stream;

mod bindings {
    wit_bindgen::generate!({ world: "exports", with: {
        "wasi:clocks/system-clock@0.3.0": generate,
        "wasi:clocks/types@0.3.0": generate,
        "wasi:cli/types@0.3.0": generate,
        "wasi:cli/stdout@0.3.0": generate,
        "wasi:cli/stderr@0.3.0": generate,
    } });
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
        stdout::write_via_stream(reader);
    } else {
        stderr::write_via_stream(reader);
    }
}

struct Component;

impl bindings::exports::wasi::logging::logging::Guest for Component {
    fn log(level: Level, context: String, message: String) {
        let level_str = match level {
            Level::Critical => "critical",
            Level::Error => "error",
            Level::Warn => "warning",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        };
        let message = format!("{}: {}", level_str, message);
        let timestamp = now();
        let duration = std::time::Duration::new(
            u64::try_from(timestamp.seconds).unwrap(),
            timestamp.nanoseconds,
        );
        let timestamp = std::time::UNIX_EPOCH + duration;
        let timestamp = chrono::DateTime::<chrono::Utc>::from(timestamp);

        println(format!("{}: {} {}", context, timestamp, message), false);
    }
}
