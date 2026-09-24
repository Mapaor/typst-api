use typst::text::{Font, FontBook};
use fontdb::Database;
use typst::utils::LazyHash;
use typst::foundations::Bytes;

pub struct FontState {
    pub book: LazyHash<FontBook>,
    pub fonts: Vec<Font>,
}

impl FontState {
    pub fn new(font_paths: &[std::path::PathBuf]) -> Self {
        let mut db = Database::new();

        // Load system fonts
        db.load_system_fonts();

        // Load user fonts
        for path in font_paths {
            if path.exists() {
                db.load_fonts_dir(path);
            } else {
                tracing::warn!("Font path does not exist: {:?}", path);
            }
        }

        let mut fonts = Vec::new();

        for face in db.faces() {
            let path = match &face.source {
                fontdb::Source::File(path) => path,
                fontdb::Source::SharedFile(path, _) => path,
                _ => continue,
            };

            if let Ok(data) = std::fs::read(path) {
                let bytes = Bytes::new(data);
                if let Some(font) = Font::new(bytes, face.index) {
                    fonts.push(font);
                }
            }
        }

        let book = FontBook::from_fonts(&fonts);

        Self {
            book: LazyHash::new(book),
            fonts,
        }
    }
}
