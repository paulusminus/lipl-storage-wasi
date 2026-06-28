use bindings::exports::wasi::cli::run;
use bindings::pm::lipl_core::types::Store;
use bindings::wasi::logging::logging::{Level, log};

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

mod bindings {
    wit_bindgen::generate!({ path: "../../wit", world: "command", generate_all });
    use super::Component;
    export!(Component);
}

struct Component;

impl run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let store = Store::new();
        if let Ok(lyrics) = store.get_lyrics().await {
            for lyric in lyrics {
                log(Level::Info, CRATE_NAME, &format!("{}", lyric.title));
            }
        }
        if let Ok(playlists) = store.get_playlists().await {
            for playlist in playlists {
                log(Level::Info, CRATE_NAME, &format!("{}", playlist.title));
            }
        }
        Ok(())
    }
}
