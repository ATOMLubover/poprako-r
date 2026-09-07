//! Canonical object-key mapping for the chapter artwork slot.

use poprako_obj_dept::key::KeyMap;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};

use crate::complex::chapter_port::artwork::ChapterArtworkComplex;
use crate::part::obj_dept::ChapterArtwork;
use crate::value::artwork::ChapterArtworkKey;

impl KeyMap for ChapterArtwork {
    // Chapter artwork key domain.
    type Dom = ChapterArtworkKey;
    // Object-storage key representation.
    type Img = String;

    // Builds the canonical chapter artwork object key.
    fn forward(value: &Self::Dom, ver: u32) -> Self::Img {
        format!("chapter_artwork/{}-{}.{}", value.chapter_id, ver, value.ext)
    }

    // Parses and validates a canonical chapter artwork object key.
    fn reverse(value: &Self::Img) -> ObjDeptRest<(Self::Dom, u32)> {
        //
        let parsed = value
            .strip_prefix("chapter_artwork/")
            .and_then(|suffix| suffix.rsplit_once('.'))
            .and_then(|(identity, ext)| {
                identity.rsplit_once('-').map(|(id, ver)| (id, ver, ext))
            })
            .and_then(|(id, ver, ext)| {
                Some((id, ver.parse::<u32>().ok()?, ext))
            })
            .filter(|(id, _, ext)| {
                //
                !id.is_empty()
                    && !id.contains('/')
                    && ChapterArtworkComplex::valid_extension(ext)
            })
            .map(|(id, ver, ext)| {
                //
                (
                    ChapterArtworkKey {
                        chapter_id: id.into(),
                        ext: ext.into(),
                    },
                    ver,
                )
            });

        parsed
            .filter(|(key, ver)| Self::forward(key, *ver) == *value)
            .ok_or_else(|| ObjDeptError::Invalid {
                message: "invalid chapter artwork key".into(),
            })
    }

    // Extracts the chapter identifier from the domain key.
    fn id(value: &Self::Dom) -> &str {
        &value.chapter_id
    }

    // Extracts the file suffix from the domain key.
    fn ext(value: &Self::Dom) -> &str {
        &value.ext
    }
}
