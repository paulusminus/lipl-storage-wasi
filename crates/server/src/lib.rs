use std::str::from_utf8;

use crate::bindings::{
    exports::wasi::http::handler::Guest,
    pm::lipl_core::types::{Lyric, Playlist, Store},
    wasi::http::types::{ErrorCode, Fields, Method, Request, Response},
};

mod bindings {
    wit_bindgen::generate!({path: "../../wit", world: "server", with: {
        "wasi:clocks/types@0.3.0": generate,
        "wasi:http/types@0.3.0": generate,
    },
    additional_derives: [serde::Serialize, serde::Deserialize]});
    use super::Component;
    export!(Component);
}

const PREFIX: &str = "api/v1/";
const LYRIC_PREFIX: &str = "lyric";
const PLAYLIST_PREFIX: &str = "playlist";

trait Json {
    fn json(self) -> Result<String, String>;
}

impl<T: serde::Serialize, E: ToString> Json for Result<T, E> {
    fn json(self) -> Result<String, String> {
        self.map_err(|e| e.to_string())
            .and_then(|s| serde_json::to_string(&s).map_err(|e| e.to_string()))
    }
}

#[derive(Debug, PartialEq)]
enum Routes {
    GetLyrics,
    GetLyric(String),
    PutLyric(String, String),
    PostLyric(String),
    DeleteLyric(String),
    GetPlaylists,
    GetPlaylist(String),
    PutPlaylist(String, String),
    PostPlaylist(String),
    DeletePlaylist(String),
}

impl TryFrom<RoutePath> for Routes {
    type Error = ErrorCode;

    fn try_from(route_path: RoutePath) -> Result<Self, Self::Error> {
        let mut path = route_path.path;
        if !path.starts_with(PREFIX) {
            return Err(ErrorCode::DestinationNotFound);
        }
        path = path.strip_prefix(PREFIX).unwrap().to_owned();
        if path.starts_with(LYRIC_PREFIX) {
            if let Some(id) = path.split('/').nth(1) {
                if id.is_empty() {
                    match route_path.method {
                        Method::Post => {
                            return Ok(Self::PostLyric(route_path.body.unwrap_or_default()));
                        }
                        Method::Get => return Ok(Self::GetLyrics),
                        _ => return Err(ErrorCode::DestinationNotFound),
                    }
                } else {
                    match route_path.method {
                        Method::Get => return Ok(Self::GetLyric(id.to_owned())),
                        Method::Put => {
                            return Ok(Self::PutLyric(
                                id.to_owned(),
                                route_path.body.unwrap_or_default(),
                            ));
                        }
                        Method::Delete => return Ok(Self::DeleteLyric(id.to_owned())),
                        _ => return Err(ErrorCode::DestinationNotFound),
                    }
                }
            } else {
                match route_path.method {
                    Method::Post => {
                        return Ok(Self::PostLyric(route_path.body.unwrap_or_default()));
                    }
                    Method::Get => return Ok(Self::GetLyrics),
                    _ => return Err(ErrorCode::DestinationNotFound),
                }
            }
        } else if path.starts_with(PLAYLIST_PREFIX) {
            if let Some(id) = path.split('/').nth(1) {
                if id.is_empty() {
                    match route_path.method {
                        Method::Post => {
                            return Ok(Self::PostPlaylist(route_path.body.unwrap_or_default()));
                        }
                        Method::Get => return Ok(Self::GetPlaylists),
                        _ => return Err(ErrorCode::DestinationNotFound),
                    }
                } else {
                    match route_path.method {
                        Method::Get => return Ok(Self::GetPlaylist(id.to_owned())),
                        Method::Put => {
                            return Ok(Self::PutPlaylist(
                                id.to_owned(),
                                route_path.body.unwrap_or_default(),
                            ));
                        }
                        Method::Delete => return Ok(Self::DeletePlaylist(id.to_owned())),
                        _ => return Err(ErrorCode::DestinationNotFound),
                    }
                }
            } else {
                return Ok(Self::GetPlaylists);
            }
        } else {
            return Err(ErrorCode::DestinationNotFound);
        }
    }
}

struct RoutePath {
    method: Method,
    path: String,
    body: Option<String>,
}

struct Component;

impl Guest for Component {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let method = request.get_method();
        let path = request
            .get_path_with_query()
            .ok_or(ErrorCode::DestinationNotFound)?;

        let (_w, r) = bindings::wit_future::new(|| todo!());
        let (incoming_body, trailer) = Request::consume_body(request, r);
        let body = incoming_body.collect().await;
        let result = trailer.await.map(|_| body)?;
        let s =
            from_utf8(result.as_slice()).map_err(|e| ErrorCode::InternalError(Some(e.to_string())));

        let route_path = RoutePath {
            method,
            path,
            body: s.map(String::from).ok(),
        };
        let route = Routes::try_from(route_path)?;

        let store = Store::new();
        let body = match route {
            Routes::GetLyrics => store.get_lyrics().await.json(),
            Routes::GetLyric(id) => store.get_lyric(id).await.json(),
            Routes::PutLyric(_, body) => {
                let lyric = serde_json::from_str::<Lyric>(&body)
                    .map_err(|e| ErrorCode::InternalError(Some(e.to_string())))?;
                store.upsert_lyric(lyric).await.json()
            }
            Routes::PostLyric(body) => {
                let lyric = serde_json::from_str::<Lyric>(&body)
                    .map_err(|e| ErrorCode::InternalError(Some(e.to_string())))?;
                store.upsert_lyric(lyric).await.json()
            }
            Routes::DeleteLyric(id) => store.delete_lyric(id).await.json(),
            Routes::GetPlaylists => store.get_playlists().await.json(),
            Routes::GetPlaylist(id) => store.get_playlist(id).await.json(),
            Routes::PutPlaylist(_, body) => {
                let playlist = serde_json::from_str::<Playlist>(&body)
                    .map_err(|e| ErrorCode::InternalError(Some(e.to_string())))?;
                store.upsert_playlist(playlist).await.json()
            }
            Routes::PostPlaylist(body) => {
                let playlist = serde_json::from_str::<Playlist>(&body)
                    .map_err(|e| ErrorCode::InternalError(Some(e.to_string())))?;
                store.upsert_playlist(playlist).await.json()
            }
            Routes::DeletePlaylist(id) => store.delete_playlist(id).await.json(),
        }
        .map_err(|s| ErrorCode::InternalError(Some(s)))?;

        let headers = Fields::new();
        headers
            .set("content-type", &["application/json".as_bytes().to_vec()])
            .unwrap();

        let (mut tx, rx) = bindings::wit_stream::new::<u8>();
        let (trailers_tx, trailers_rx) = bindings::wit_future::new(|| todo!());

        wit_bindgen::spawn_local(async move {
            tx.write_all(body.as_bytes().to_vec()).await;
            drop(tx);
            let _ = trailers_tx.write(Ok(None)).await;
        });

        let (response, _result) = Response::new(headers, Some(rx), trailers_rx);
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::{Method, RoutePath, Routes};

    #[test]
    fn test_routes() {
        assert_eq!(
            Routes::try_from(RoutePath {
                method: Method::Get,
                path: "api/v1/lyric".to_string(),
                body: None,
            })
            .unwrap(),
            Routes::GetLyrics
        );
        assert_eq!(
            Routes::try_from(RoutePath {
                method: Method::Get,
                path: "api/v1/lyric/".to_string(),
                body: None,
            })
            .unwrap(),
            Routes::GetLyrics
        );
        assert_eq!(
            Routes::try_from(RoutePath {
                method: Method::Get,
                path: "api/v1/lyric/1".to_string(),
                body: None,
            })
            .unwrap(),
            Routes::GetLyric("1".to_string())
        );
        assert_eq!(
            Routes::try_from(RoutePath {
                method: Method::Post,
                path: "api/v1/lyric".to_string(),
                body: Some("lyric body".to_string()),
            })
            .unwrap(),
            Routes::PostLyric("lyric body".to_string())
        );
        assert_eq!(
            Routes::try_from(RoutePath {
                method: Method::Put,
                path: "api/v1/lyric/1".to_string(),
                body: Some("lyric body".to_string()),
            })
            .unwrap(),
            Routes::PutLyric("1".to_string(), "lyric body".to_string())
        );
        assert_eq!(
            Routes::try_from(RoutePath {
                method: Method::Delete,
                path: "api/v1/lyrics/1".to_string(),
                body: None,
            })
            .unwrap(),
            Routes::DeleteLyric("1".to_string())
        );
    }
}
