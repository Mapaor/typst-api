use std::collections::HashMap;
use std::sync::Arc;
use chrono::{Datelike, Local};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, Source, VirtualPath, RootedPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst::LibraryExt;

use crate::fonts::FontState;

pub struct ApiWorld {
    pub library: LazyHash<Library>,
    pub font_state: Arc<FontState>,
    pub main: FileId,
    pub files: HashMap<FileId, Bytes>,
    pub sources: HashMap<FileId, Source>,
    pub today: Datetime,
}

impl ApiWorld {
    pub fn new(font_state: Arc<FontState>, files_data: HashMap<String, Vec<u8>>, main_file: String) -> Result<Self, String> {
        let library = LazyHash::new(Library::builder().build());

        let mut files = HashMap::new();
        let mut sources = HashMap::new();
        
        for (name, data) in files_data {
            let vpath = VirtualPath::new(&name).map_err(|e| format!("Invalid path '{}': {}", name, e))?;
            let rooted = RootedPath::new(VirtualRoot::Project, vpath);
            let id = FileId::new(rooted);
            let bytes = Bytes::new(data.clone());
            files.insert(id, bytes.clone());
            
            if name.ends_with(".typ") {
                if let Ok(text) = String::from_utf8(data) {
                    sources.insert(id, Source::new(id, text));
                }
            }
        }

        let main_vpath = VirtualPath::new(&main_file).map_err(|e| format!("Invalid path '{}': {}", main_file, e))?;
        let main_rooted = RootedPath::new(VirtualRoot::Project, main_vpath);
        let main_id = FileId::new(main_rooted);

        if !sources.contains_key(&main_id) {
            return Err(format!("Main file '{}' not found or not valid utf-8", main_file));
        }

        let now = Local::now();
        let today = Datetime::from_ymd(now.year(), now.month() as u8, now.day() as u8)
            .unwrap_or(Datetime::from_ymd(1970, 1, 1).unwrap());

        Ok(Self {
            library,
            font_state,
            main: main_id,
            files,
            sources,
            today,
        })
    }
}

impl World for ApiWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.font_state.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.sources.get(&id).cloned().ok_or_else(|| FileError::NotFound(std::path::PathBuf::from(id.vpath().get_without_slash())))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or_else(|| FileError::NotFound(std::path::PathBuf::from(id.vpath().get_without_slash())))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.font_state.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Some(self.today)
    }
}
