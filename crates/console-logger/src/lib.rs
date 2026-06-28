use bindings::exports::wasi::logging::logging::Level;
use bindings::wasi::clocks::system_clock::now;

mod bindings {
    wit_bindgen::generate!({ generate_all });
    use super::Component;
    export!(Component);
}

struct Component;

impl bindings::exports::wasi::logging::logging::Guest for Component {
    fn log(level: Level, _: String, message: String) {
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

        println!("{} {}", timestamp, message);
    }
}
