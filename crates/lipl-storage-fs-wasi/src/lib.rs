#![warn(clippy::pedantic)]
#![allow(async_fn_in_trait)]
use std::str::FromStr;

use crate::{
    bindings::exports::pm::lipl_core::types::{Error, GuestStore, Lyric, Playlist},
    constant::{MARKDOWN_EXTENSION, TOML_EXTENSION},
    error::ErrInto,
    lib_ext::Directory,
    part::extract_delimited_frontmatter,
};
use bindings::wasi::filesystem::types::Descriptor;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
mod constant;
mod error;
mod lib_ext;
mod part;

mod bindings {
    wit_bindgen::generate!({ path: "../../wit", world: "storage-fs", generate_all });
    use super::Component;
    export!(Component);
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LyricMeta {
    pub title: String,
    pub hash: String,
}

pub struct LyricPost {
    pub title: String,
    pub parts: Vec<Vec<String>>,
}

impl FromStr for LyricPost {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (frontmatter, content) = extract_delimited_frontmatter(s)?;
        let meta: LyricMeta = toml::from_str(frontmatter)?;
        Ok(Self {
            title: meta.title,
            parts: part::to_parts(content),
        })
    }
}

struct Store {
    directory: Directory,
}

struct Component;

impl bindings::exports::pm::lipl_core::types::Guest for Component {
    type Store = Store;
}

#[allow(dead_code)]
async fn get_content(file: &Descriptor) -> Result<String, Error> {
    let (s, terminate) = file.read_via_stream(0);
    let contents: Vec<u8> = s.collect().await;
    terminate.await?;
    String::from_utf8(contents).err_into()
}

impl GuestStore for Store {
    fn new() -> Self {
        Self {
            directory: Directory::new_root().unwrap(),
        }
    }

    async fn get_lyrics(&self) -> Result<Vec<Lyric>, Error> {
        let lyric_files = self
            .directory
            .get_files::<Lyric>(MARKDOWN_EXTENSION)
            .await?;
        lyric_files.into_iter().map(Lyric::try_from).collect()
    }

    async fn get_lyric(&self, id: String) -> Result<Lyric, Error> {
        self.directory
            .open_file::<Lyric>(format!("{id}{MARKDOWN_EXTENSION}"))
            .await
            .and_then(TryFrom::try_from)
    }

    async fn upsert_lyric(&self, _lyric: Lyric) -> Result<(), Error> {
        todo!()
    }

    async fn delete_lyric(&self, id: String) -> Result<(), Error> {
        self.directory
            .delete_entry(format!("{id}{MARKDOWN_EXTENSION}"))
            .await
    }

    async fn get_playlists(&self) -> Result<Vec<Playlist>, Error> {
        let lyric_files = self.directory.get_files::<Playlist>(TOML_EXTENSION).await?;
        lyric_files.into_iter().map(TryFrom::try_from).collect()
    }

    async fn get_playlist(&self, id: String) -> Result<Playlist, Error> {
        self.directory
            .open_file::<Playlist>(format!("{id}{TOML_EXTENSION}"))
            .await
            .and_then(TryFrom::try_from)
    }

    async fn upsert_playlist(&self, _playlist: Playlist) -> Result<(), Error> {
        todo!()
    }

    async fn delete_playlist(&self, id: String) -> Result<(), Error> {
        self.directory
            .delete_entry(format!("{}{}", id, TOML_EXTENSION))
            .await
    }
}
