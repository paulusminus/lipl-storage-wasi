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
        match store.get_lyrics().await {
            Ok(lyrics) => {
                log(
                    Level::Info,
                    CRATE_NAME,
                    &format!("found {} lyrics", lyrics.len()),
                );
                for mut lyric in lyrics {
                    log(
                        Level::Info,
                        CRATE_NAME,
                        &format!("{} ({})", &lyric.title, &lyric.id),
                    );
                    if lyric.id.as_str() == "QKKvuNZBAph1JaHLs3UNtu" {
                        lyric.title = "Oh kleintje".to_owned();
                        if store.upsert_lyric(lyric).await.is_ok() {
                            log(
                                Level::Info,
                                CRATE_NAME,
                                &format!("Updated lyric title to 'Vader Jacobje'"),
                            );
                        };
                    }
                }
            }
            Err(error) => {
                log(
                    Level::Error,
                    CRATE_NAME,
                    &format!("failed to get lyrics: {}", error),
                );
            }
        }
        if let Ok(playlists) = store.get_playlists().await {
            for playlist in playlists {
                log(
                    Level::Info,
                    CRATE_NAME,
                    &format!("{} ({})", playlist.title, playlist.id),
                );
            }
        }
        Ok(())
    }
}
