use serde::Deserialize;
use std::{marker::PhantomData, path::Path};
use wit_bindgen::block_on;

use crate::bindings::exports::pm::lipl_core::types::{Error, Lyric, Playlist};
use crate::{
    ErrInto, LyricMeta,
    bindings::wasi::filesystem::{
        preopens,
        types::{
            Descriptor, DescriptorFlags, DescriptorType, DirectoryEntry, OpenFlags, PathFlags,
        },
    },
    constant::DEFAULT_PREOPEN_PATH,
    part::{extract_delimited_frontmatter, to_parts},
};

#[allow(dead_code)]
fn is_regular_file(entry: &DirectoryEntry) -> bool {
    matches!(entry.type_, DescriptorType::RegularFile)
}

#[allow(dead_code)]
fn is_directory(entry: &DirectoryEntry) -> bool {
    matches!(entry.type_, DescriptorType::Directory)
}

pub fn file_has_suffix(suffix: &str) -> impl Fn(&DirectoryEntry) -> bool {
    move |entry| is_regular_file(entry) && entry.name.ends_with(suffix)
}

#[allow(dead_code)]
pub struct File<T> {
    pub name: String,
    pub descriptor: Descriptor,
    _phantom: PhantomData<T>,
}

impl TryFrom<File<Lyric>> for Lyric {
    type Error = Error;

    fn try_from(file: File<Lyric>) -> Result<Self, Self::Error> {
        let content = block_on(file.contents())?;
        let (frontmatter, text) = extract_delimited_frontmatter(&content)?;
        let lyric_meta = toml::from_str::<LyricMeta>(frontmatter)?;
        Ok(Lyric {
            id: file.id(),
            title: lyric_meta.title,
            parts: to_parts(text),
        })
    }
}

#[derive(Deserialize)]
struct PlaylistPost {
    title: String,
    members: Vec<String>,
}

impl TryFrom<File<Playlist>> for Playlist {
    type Error = crate::bindings::exports::pm::lipl_core::types::Error;

    fn try_from(file: File<Playlist>) -> Result<Self, Self::Error> {
        let content = block_on(file.contents())?;
        let playlist_post: PlaylistPost = toml::from_str(&content).err_into()?;
        Ok(Playlist {
            id: file.id(),
            title: playlist_post.title,
            members: playlist_post.members,
        })
    }
}

#[allow(dead_code)]
impl<T> File<T> {
    pub async fn contents(&self) -> Result<String, Error> {
        let (stream, terminate) = self.descriptor.read_via_stream(0);
        let contents = stream.collect().await;
        terminate.await?;
        String::from_utf8(contents).err_into()
    }

    pub fn id(&self) -> String {
        Path::new(&self.name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }
}

#[allow(dead_code)]
pub struct Directory {
    pub name: String,
    pub descriptor: Descriptor,
}

impl From<(Descriptor, String)> for Directory {
    fn from((descriptor, name): (Descriptor, String)) -> Self {
        Directory { name, descriptor }
    }
}

#[allow(dead_code)]
impl Directory {
    pub fn new_root() -> Result<Self, Error> {
        preopens::get_directories()
            .into_iter()
            .find(|(_, path)| path.as_str() == DEFAULT_PREOPEN_PATH)
            .map(Into::into)
            .ok_or(Error::Io(format!(
                "No preopen found with path {}",
                DEFAULT_PREOPEN_PATH
            )))
    }

    pub async fn delete_entry(&self, name: String) -> Result<(), Error> {
        self.descriptor.unlink_file_at(name).await.err_into()
    }

    pub async fn get_entry(&self, name: String) -> Result<DirectoryEntry, Error> {
        let entries = self
            .get_entries(|entry| is_regular_file(entry) && entry.name == name)
            .await?;
        entries
            .first()
            .cloned()
            .ok_or(Error::Io("No entry found".to_string()))
    }

    pub async fn get_entries<F>(&self, filter: F) -> Result<Vec<DirectoryEntry>, Error>
    where
        F: Fn(&DirectoryEntry) -> bool,
    {
        let (stream, terminate) = self.descriptor.read_directory();
        let entries = stream.collect().await;
        terminate
            .await
            .map(|_| entries.into_iter().filter(filter).collect())
            .map_err(|error_code| Error::Io(error_code.to_string()))
    }

    pub async fn open_file<T>(&self, name: String) -> Result<File<T>, Error> {
        self.descriptor
            .open_at(
                PathFlags::empty(),
                name.clone(),
                OpenFlags::empty(),
                DescriptorFlags::READ,
            )
            .await
            .err_into()
            .map(|descriptor| File {
                name,
                descriptor,
                _phantom: PhantomData,
            })
    }

    // pub async fn get_file<T: 'static>(&self, name: &str) -> Result<File<T>, Error> {
    //     self.open_file::<T>(name.to_owned()).await
    // }

    pub async fn get_files<T: 'static>(&self, suffix: &str) -> Result<Vec<File<T>>, Error> {
        let filter = file_has_suffix(suffix);
        let entries = self.get_entries(filter).await?;
        entries
            .into_iter()
            .map(|entry| block_on(self.open_file::<T>(entry.name)))
            .collect()
    }
}
