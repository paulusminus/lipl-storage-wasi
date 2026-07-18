#![warn(clippy::pedantic)]
#![allow(async_fn_in_trait)]
use std::{
    str::FromStr,
    sync::{LazyLock, OnceLock, RwLock},
};

use crate::{
    bindings::exports::pm::lipl_core::types::{Error, Lyric, Playlist},
    constant::{MARKDOWN_EXTENSION, TOML_EXTENSION},
    error::ErrInto,
    lib_ext::Directory,
    part::{extract_delimited_frontmatter, to_text},
};
use bindings::wasi::filesystem::types::Descriptor;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
mod constant;
mod error;
mod lib_ext;
mod part;

#[allow(dead_code)]
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

mod bindings {
    wit_bindgen::generate!({
        path: "../../wit",
        world: "storage-fs",
        with: {
            "wasi:clocks/types@0.3.0": generate,
            "wasi:clocks/system-clock@0.3.0": generate,
            "wasi:filesystem/types@0.3.0": generate,
            "wasi:filesystem/preopens@0.3.0": generate,
        },
        additional_derives: [serde::Deserialize, serde::Serialize]
    });
    use super::Component;
    export!(Component);
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LyricMeta {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
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

struct Component;

impl bindings::exports::pm::lipl_core::types::Guest for Component {
    async fn get_lyrics() -> Result<Vec<Lyric>, Error> {
        increment();
        let directory = DIRECTORY.get_or_init(|| Directory::new_root().unwrap());
        let lyric_files = directory.get_files::<Lyric>(MARKDOWN_EXTENSION).await?;
        lyric_files.into_iter().map(TryFrom::try_from).collect()
    }

    async fn get_lyric(id: String) -> Result<Lyric, Error> {
        increment();
        let directory = DIRECTORY.get_or_init(|| Directory::new_root().unwrap());
        directory
            .open_file::<Lyric>(format!("{id}{MARKDOWN_EXTENSION}"), false)
            .await
            .and_then(TryFrom::try_from)
    }

    async fn upsert_lyric(lyric: Lyric) -> Result<(), Error> {
        increment();
        let directory = DIRECTORY.get_or_init(|| Directory::new_root().unwrap());
        let id = lyric.id.clone();
        let lyric_meta = LyricMeta {
            title: lyric.title.clone(),
            hash: None,
        };
        let lyric_meta_toml = toml::to_string_pretty(&lyric_meta).unwrap();
        let parts = to_text(&lyric.parts);
        let content = format!("+++\n{}+++\n\n{}", lyric_meta_toml, parts);
        let file = directory
            .open_file::<Lyric>(format!("{id}{MARKDOWN_EXTENSION}"), true)
            .await?;
        file.write_contents(content).await
    }

    async fn delete_lyric(id: String) -> Result<(), Error> {
        increment();
        let directory = DIRECTORY.get_or_init(|| Directory::new_root().unwrap());
        for mut playlist in Component::get_playlists().await? {
            if playlist.members.contains(&id) {
                playlist.members.retain(|l| l != &id);
                Component::upsert_playlist(playlist).await?;
            }
        }
        directory
            .delete_entry(format!("{id}{MARKDOWN_EXTENSION}"))
            .await
    }

    async fn get_playlists() -> Result<Vec<Playlist>, Error> {
        increment();
        let directory = DIRECTORY.get_or_init(|| Directory::new_root().unwrap());
        let lyric_files = directory.get_files::<Playlist>(TOML_EXTENSION).await?;
        lyric_files.into_iter().map(TryFrom::try_from).collect()
    }

    async fn get_playlist(id: String) -> Result<Playlist, Error> {
        increment();
        let directory = DIRECTORY.get_or_init(|| Directory::new_root().unwrap());
        directory
            .open_file::<Playlist>(format!("{id}{TOML_EXTENSION}"), false)
            .await
            .and_then(TryFrom::try_from)
    }

    async fn upsert_playlist(playlist: Playlist) -> Result<(), Error> {
        increment();
        let id = playlist.id.clone();
        let content = toml::to_string_pretty(&playlist).unwrap();
        let directory = DIRECTORY.get_or_init(|| Directory::new_root().unwrap());
        let file = directory
            .open_file::<Playlist>(format!("{id}{TOML_EXTENSION}"), true)
            .await?;
        file.write_contents(content).await
    }

    async fn delete_playlist(id: String) -> Result<(), Error> {
        increment();
        let directory = DIRECTORY.get_or_init(|| Directory::new_root().unwrap());
        directory
            .delete_entry(format!("{}{}", id, TOML_EXTENSION))
            .await
    }

    fn get_count() -> u64 {
        read()
    }
}

#[allow(dead_code)]
async fn get_content(file: &Descriptor) -> Result<String, Error> {
    let (s, terminate) = file.read_via_stream(0);
    let contents: Vec<u8> = s.collect().await;
    terminate.await?;
    String::from_utf8(contents).err_into()
}

static DIRECTORY: OnceLock<Directory> = OnceLock::new();

fn increment() {
    let mut lock = TEST.write().unwrap();
    *lock = lock.saturating_add(1);
}

fn read() -> u64 {
    *TEST.read().unwrap()
}

static TEST: LazyLock<RwLock<u64>> = LazyLock::new(|| RwLock::new(0));
