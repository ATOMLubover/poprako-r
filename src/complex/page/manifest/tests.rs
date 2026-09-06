use super::*;

use crate::value::image::{ImageExt, ImageHash};

fn candidate(
    id: &'static str,
    index: usize,
    has_units: bool,
    is_image_available: bool,
    ext: &'static str,
) -> PageManifestCand<'static> {
    PageManifestCand {
        id,
        chapter_id: "chapter-1",
        index,
        has_units,
        image_uploaded: is_image_available,
        image_hash: Some(&[0; 32]),
        image_ext: Some(ext),
    }
}

fn input(page_id: Option<&str>, hash: u8, ext: ImageExt) -> PageImageSpec {
    PageImageSpec {
        raw_ident: None,
        page_id: page_id.map(Into::into),
        image_hash: ImageHash::new([hash; 32]),
        new_byte_len: Some(4096),
        ext,
    }
}

#[test]
fn explicit_identity_is_reserved_before_automatic_matching() {
    let candidates = vec![
        candidate("page-a", 0, false, true, "png"),
        candidate("page-b", 1, true, true, "png"),
    ];
    let inputs = vec![
        input(None, 0, ImageExt::Png),
        input(Some("page-b"), 0, ImageExt::Png),
    ];

    let plan =
        PageManifestComplex::build("chapter-1", &candidates, &inputs).unwrap();

    assert_eq!(plan.matches[0].existing_index, Some(0));
    assert_eq!(plan.matches[1].existing_index, Some(1));
}

#[test]
fn automatic_matching_uses_units_availability_index_and_id_priority() {
    let candidates = vec![
        candidate("page-z", 0, false, true, "png"),
        candidate("page-b", 3, true, false, "png"),
        candidate("page-c", 2, true, true, "png"),
        candidate("page-a", 2, true, true, "png"),
    ];
    let inputs = vec![
        input(None, 0, ImageExt::Png),
        input(None, 0, ImageExt::Png),
        input(None, 0, ImageExt::Png),
    ];

    let plan =
        PageManifestComplex::build("chapter-1", &candidates, &inputs).unwrap();

    assert_eq!(plan.matches[0].existing_index, Some(3));
    assert_eq!(plan.matches[1].existing_index, Some(2));
    assert_eq!(plan.matches[2].existing_index, Some(1));
    assert_eq!(plan.deleted_existing_indexes, vec![0]);
}

#[test]
fn explicit_identity_can_replace_the_image_hash() {
    let candidates = vec![candidate("page-a", 0, true, true, "png")];
    let inputs = vec![input(Some("page-a"), 1, ImageExt::Png)];

    let plan =
        PageManifestComplex::build("chapter-1", &candidates, &inputs).unwrap();

    assert_eq!(plan.matches[0].existing_index, Some(0));
    assert!(plan.deleted_existing_indexes.is_empty());
}

#[test]
fn same_hash_with_different_extension_is_a_distinct_identity() {
    let candidates = vec![candidate("page-a", 0, false, true, "png")];
    let inputs = vec![input(None, 0, ImageExt::Jpg)];

    let plan =
        PageManifestComplex::build("chapter-1", &candidates, &inputs).unwrap();

    assert!(plan.matches[0].existing_index.is_none());
    assert_eq!(plan.deleted_existing_indexes, vec![0]);
}
