//! Total ObjDept composition.

// Chapter artwork storage-key mapping.
mod artwork;

#[cfg(test)]
mod mock_impl;

/// R2 object-storage implementation.
pub mod r2_impl;

#[cfg(all(test, feature = "rdb"))]
pub mod tests;

use poprako_obj_dept::key::KeyMap;
use poprako_obj_dept::pool::{ObjDeptPool, ObjDeptPoolView};
use poprako_obj_dept::prom::ObjDeptProm;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};
use poprako_obj_dept::{impl_obj_dept, objs_def, rdb_obj_dept_prom};
use poprako_rdb_core::RdbCore;

#[cfg(test)]
use crate::implement_mock_obj_dept;

use crate::complex::image::ImageComplex;
use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageImage, TeamAvatar, UserAvatar,
};
use crate::part_impl::obj_dept::r2_impl::R2ObjDeptPool;
use crate::part_impl::repo::rdb_impl::schema::{
    t_chapter_artwork, t_comic_cover, t_obj_prom_task, t_page_image,
    t_team_avatar, t_user_avatar,
};
use crate::value::image::{
    ComicCoverKey, PageImageKey, TeamAvatarKey, UserAvatarKey,
};

impl KeyMap for PageImage {
    // Business identity used by page-image keys.
    type Dom = PageImageKey;
    // Complete page-image storage key.
    type Img = String;

    // Returns the page identifier persisted in the object table.
    fn id(value: &Self::Dom) -> &str {
        &value.page_id
    }

    // Returns the validated image extension.
    fn ext(value: &Self::Dom) -> &str {
        value.ext.suffix()
    }

    // Builds the canonical page-image key.
    fn forward(value: &Self::Dom, ver: u32) -> Self::Img {
        ImageComplex::page_key(value, ver)
    }

    // Parses the canonical page-image key.
    fn reverse(value: &Self::Img) -> ObjDeptRest<(Self::Dom, u32)> {
        //
        ImageComplex::parse_page_key(value)
            .ok_or_else(|| invalid_key("page image"))
    }
}

// Implements a flat-key mapping for a non-page image kind.
macro_rules! impl_flat_key_map {
    ($marker:ty, $dom:ty, $id:ident, $kind:literal, $forward:path, $reverse:path) => {
        impl KeyMap for $marker {
            type Dom = $dom;
            type Img = String;

            fn id(value: &Self::Dom) -> &str {
                &value.$id
            }

            fn ext(value: &Self::Dom) -> &str {
                value.ext.suffix()
            }

            fn forward(value: &Self::Dom, ver: u32) -> Self::Img {
                $forward(value, ver)
            }

            fn reverse(value: &Self::Img) -> ObjDeptRest<(Self::Dom, u32)> {
                $reverse(value).ok_or_else(|| invalid_key($kind))
            }
        }
    };
}

impl_flat_key_map!(
    UserAvatar,
    UserAvatarKey,
    user_id,
    "user avatar",
    ImageComplex::user_avatar_key,
    ImageComplex::parse_user_avatar_key
);

impl_flat_key_map!(
    TeamAvatar,
    TeamAvatarKey,
    team_id,
    "team avatar",
    ImageComplex::team_avatar_key,
    ImageComplex::parse_team_avatar_key
);

impl_flat_key_map!(
    ComicCover,
    ComicCoverKey,
    comic_id,
    "comic cover",
    ImageComplex::comic_cover_key,
    ImageComplex::parse_comic_cover_key
);

rdb_obj_dept_prom! {
    //
    RdbObjDeptProm {
        table: t_obj_prom_task,
    }
}

objs_def! {
    ChapterArtwork {
        table: t_chapter_artwork,
        topic: "chapter_artwork",
    },
    PageImage {
        table: t_page_image,
        topic: "page_image",
    },
    UserAvatar {
        table: t_user_avatar,
        topic: "user_avatar",
    },
    TeamAvatar {
        table: t_team_avatar,
        topic: "team_avatar",
    },
    ComicCover {
        table: t_comic_cover,
        topic: "comic_cover",
    },
}

/// Total object department composed from storage and durable-task adapters.
pub struct NormObjDept<P = R2ObjDeptPool, M = RdbObjDeptProm> {
    //
    /// Shared relational database core.
    core: RdbCore,
    /// Physical object-storage adapter.
    pool: P,
    /// Durable object-task adapter.
    prom: M,
}

impl<P, M> NormObjDept<P, M> {
    /// Constructs the department from already composed storage adapters.
    pub const fn new(core: RdbCore, pool: P, prom: M) -> Self {
        Self { core, pool, prom }
    }
}

impl<P, M> NormObjDept<P, M>
where
    P: Clone,
{
    /// Returns a read-only projection sharing the injected storage dependencies.
    pub fn view(&self) -> NormObjDeptView<P> {
        //
        NormObjDeptView {
            core: self.core.clone(),
            pool: self.pool.clone(),
        }
    }
}

impl<P, M> NormObjDept<P, M>
where
    P: ObjDeptPool,
    M: ObjDeptProm,
{
    // Returns the shared relational database core.
    const fn core(&self) -> &RdbCore {
        &self.core
    }

    // Returns the physical object-storage adapter.
    const fn pool(&self) -> &P {
        &self.pool
    }

    // Returns the durable object-task adapter.
    const fn prom(&self) -> &M {
        &self.prom
    }
}

impl<P, M> Clone for NormObjDept<P, M>
where
    P: Clone,
    M: Clone,
{
    // Clones the shared object department handle.
    fn clone(&self) -> Self {
        //
        Self {
            core: self.core.clone(),
            pool: self.pool.clone(),
            prom: self.prom.clone(),
        }
    }
}

/// Read-only projection of object metadata and physical storage.
#[derive(Clone)]
pub struct NormObjDeptView<P> {
    //
    /// Shared relational database core.
    core: RdbCore,
    /// Physical object-storage adapter.
    pool: P,
}

impl<P> NormObjDeptView<P>
where
    P: ObjDeptPoolView,
{
    // Returns the shared relational database core.
    const fn core(&self) -> &RdbCore {
        &self.core
    }

    // Returns the physical object-storage adapter.
    const fn pool(&self) -> &P {
        &self.pool
    }
}

impl_obj_dept! {
    dept: NormObjDept,
    view: NormObjDeptView,
}

// Expands test adapters from the object manifest.
#[cfg(test)]
macro_rules! implement_mock_obj_dept_from_manifest {
    ($(($marker:ident, $module:ident, $topic:literal),)*) => {
        $(implement_mock_obj_dept!($marker, $topic);)*
    };
}

#[cfg(test)]
for_each_obj!(implement_mock_obj_dept_from_manifest);

// Builds a stable invalid-key error at the concrete mapping boundary.
fn invalid_key(kind: &str) -> ObjDeptError {
    //
    ObjDeptError::Invalid {
        message: format!("invalid {} physical key", kind),
    }
}
