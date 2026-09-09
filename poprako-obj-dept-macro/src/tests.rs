use proc_macro2::TokenStream;
use quote::quote;

use crate::{impl_obj_dept, object};

// Returns the declaration error text, or an empty string when expansion succeeds.
fn expand_err(input: TokenStream) -> String {
    match object::expand(input) {
        Ok(_) => String::new(),
        Err(err) => err.to_string(),
    }
}

#[test]
fn object_manifest_rejects_every_duplicate_identity() {
    //
    let duplicate_marker = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
        },
        PageImage {
            table: t_page_image_2,
            topic: "page_image_2",
        },
    });

    let duplicate_table = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
        },
        ComicCover {
            table: t_page_image,
            topic: "comic_cover",
        },
    });

    let duplicate_topic = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
        },
        ComicCover {
            table: t_comic_cover,
            topic: "page_image",
        },
    });

    assert_eq!(duplicate_marker, "duplicate object marker");

    assert_eq!(duplicate_table, "duplicate object table");

    assert_eq!(duplicate_topic, "duplicate object topic");
}

#[test]
fn generated_names_are_readable_and_read_views_cover_every_marker()
-> syn::Result<()> {
    let manifest = object::expand(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
        },
        FontFile {
            table: t_font_file,
            topic: "font_file",
        },
    })?
    .to_string();
    let implementations = impl_obj_dept::expand_items(quote! {
        dept: NormObjDept,
        view: NormObjDeptView;
        (PageImage, page_image_rdb_impl, "page_image"),
        (FontFile, font_file_rdb_impl, "font_file"),
    })?
    .to_string();

    assert!(manifest.contains("page_image_rdb_impl"));

    assert!(manifest.contains("font_file_rdb_impl"));

    assert!(manifest.contains("for_each_obj"));

    assert!(!manifest.contains("__"));

    assert!(implementations.contains("NormObjDeptView"));

    assert!(implementations.contains("PageImage"));

    assert!(implementations.contains("FontFile"));

    assert!(!implementations.contains("__"));

    assert_eq!(implementations.matches("ensure_anchors").count(), 2);

    assert!(!implementations.contains("advisory"));

    assert!(manifest.contains("on_conflict"));

    assert!(manifest.contains("do_update"));

    assert!(manifest.contains("as_returning"));

    assert!(manifest.contains("load_for_presence_reconciliation"));

    assert!(manifest.contains("f_updated_at . eq (revision)"));

    assert!(implementations.contains("rows . len () != ids . len ()"));

    Ok(())
}
