#![warn(clippy::pedantic)]
#![allow(async_fn_in_trait)]
use std::str::FromStr;

use crate::{
    bindings::exports::pm::lipl_core::types::{Error, Lyric, Playlist},
    constant::{MARKDOWN_EXTENSION, TOML_EXTENSION},
    error::ErrInto,
    lib_ext::Directory,
    part::{extract_delimited_frontmatter, to_text},
};
use bindings::wasi::filesystem::types::Descriptor;
use serde::{Deserialize, Serialize};
use toml::Table;

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

trait Etag {
    fn etag(&self) -> String;
}

impl<T: Serialize> Etag for T {
    fn etag(&self) -> String {
        let s = toml::to_string(self).unwrap();
        etag::EntityTag::const_from_data(s.as_bytes()).to_string()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LyricMeta {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl From<Lyric> for LyricMeta {
    fn from(lyric: Lyric) -> Self {
        let hash = lyric.etag();
        Self {
            title: lyric.title,
            hash: Some(hash),
        }
    }
}

pub struct LyricPost {
    pub title: String,
    pub parts: Vec<Vec<String>>,
}

#[allow(dead_code)]
fn from_playlist_to_toml(playlist: &Playlist) -> Result<String, Error> {
    let mut toml = toml::Table::new();
    toml.insert(
        "title".to_owned(),
        toml::Value::String(playlist.title.clone()),
    );
    toml.insert(
        "members".to_owned(),
        toml::Value::Array(
            playlist
                .members
                .iter()
                .map(|m| toml::Value::String(m.clone()))
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    toml::to_string(&toml).map_err(|_| Error::Parse)
}

#[allow(dead_code)]
fn from_toml_to_playlist(s: &str, id: String) -> Result<Playlist, Error> {
    let post: Table = toml::from_str(s)?;
    let title = post
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or(Error::Parse)?;
    let members = post
        .get("members")
        .and_then(|v| v.as_array())
        .ok_or(Error::Parse)?;
    let members = members
        .iter()
        .map(|v| v.as_str().map(String::from).ok_or(Error::Parse))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Playlist {
        id,
        title: title.to_owned(),
        members,
    })
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
        let directory = Directory::new_root()?;
        let lyric_files = directory.get_files::<Lyric>(MARKDOWN_EXTENSION).await?;
        lyric_files.into_iter().map(TryFrom::try_from).collect()
    }

    async fn get_lyric(id: String) -> Result<Lyric, Error> {
        let directory = Directory::new_root()?;
        directory
            .open_file::<Lyric>(format!("{id}{MARKDOWN_EXTENSION}"), false)
            .await
            .and_then(TryFrom::try_from)
    }

    async fn upsert_lyric(lyric: Lyric) -> Result<(), Error> {
        let directory = Directory::new_root()?;
        let id = lyric.id.clone();
        let parts = to_text(&lyric.parts);
        let lyric_meta: LyricMeta = lyric.into();
        let lyric_meta_toml = toml::to_string_pretty(&lyric_meta).unwrap();
        let content = format!("+++\n{lyric_meta_toml}+++\n\n{parts}");
        let file = directory
            .open_file::<Lyric>(format!("{id}{MARKDOWN_EXTENSION}"), true)
            .await?;
        file.write_contents(content).await
    }

    async fn delete_lyric(id: String) -> Result<(), Error> {
        let directory = Directory::new_root()?;
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
        let directory = Directory::new_root()?;
        let lyric_files = directory.get_files::<Playlist>(TOML_EXTENSION).await?;
        lyric_files.into_iter().map(TryFrom::try_from).collect()
    }

    async fn get_playlist(id: String) -> Result<Playlist, Error> {
        let directory = Directory::new_root()?;
        directory
            .open_file::<Playlist>(format!("{id}{TOML_EXTENSION}"), false)
            .await
            .and_then(TryFrom::try_from)
    }

    async fn upsert_playlist(playlist: Playlist) -> Result<(), Error> {
        let id = playlist.id.clone();
        let content = toml::to_string_pretty(&playlist).unwrap();
        let directory = Directory::new_root()?;
        let file = directory
            .open_file::<Playlist>(format!("{id}{TOML_EXTENSION}"), true)
            .await?;
        file.write_contents(content).await
    }

    async fn delete_playlist(id: String) -> Result<(), Error> {
        let directory = Directory::new_root()?;
        directory
            .delete_entry(format!("{id}{TOML_EXTENSION}"))
            .await
    }

    fn get_count() -> u64 {
        Default::default()
    }
}

#[allow(dead_code)]
async fn get_content(file: &Descriptor) -> Result<String, Error> {
    let (s, terminate) = file.read_via_stream(0);
    let contents: Vec<u8> = s.collect().await;
    terminate.await?;
    String::from_utf8(contents).err_into()
}
