//! Total ObjDept composition.

// Chapter artwork storage-key mapping.
mod artwork;

#[cfg(test)]
mod mock_impl;

// R2 object-storage implementation.
mod r2_impl;

#[cfg(all(test, feature = "rdb"))]
pub mod tests;

use poprako_obj_dept::actor::{ObjActor, ObjActorDesc};
use poprako_obj_dept::key::KeyMap;
use poprako_obj_dept::pool::{ObjPool, ObjPoolView};
use poprako_obj_dept::prom::ObjProm;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};
use poprako_obj_dept::{impl_obj_dept, objs_def, rdb_obj_prom};
use poprako_rdb_core::RdbCore;

#[cfg(test)]
use crate::implement_mock_obj_dept;

use crate::complex::image::ImageComplex;
use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageImage, TeamAvatar, UserAvatar,
};
use crate::part_impl::obj_dept::r2_impl::R2ObjPool;
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

rdb_obj_prom! {
    //
    RdbObjProm {
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
pub struct NormObjDept<P = R2ObjPool, M = RdbObjProm> {
    //
    /// Shared relational database core.
    core: RdbCore,
    /// Physical object-storage adapter.
    pool: P,
    /// Durable object-task adapter.
    prom: M,
    /// Control descriptor for the single actor.
    actor_desc: ObjActorDesc,
}

/// Read-only projection of object metadata and physical storage.
#[derive(Clone)]
pub struct NormObjView<P> {
    //
    /// Shared relational database core.
    core: RdbCore,
    /// Physical object-storage adapter.
    pool: P,
}

impl<P> NormObjView<P>
where
    P: ObjPoolView,
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

impl<P, M> NormObjDept<P, M>
where
    P: ObjPool + Clone + Send + Sync + 'static,
    M: ObjProm + Clone + Send + Sync + 'static,
{
    /// Cancels the actor and waits for it to finish.
    pub async fn close(&self) {
        //
        self.actor_desc.cancel();

        self.actor_desc.join().await;
    }

    /// Returns a read-only projection without the durable object-task adapter.
    pub fn view(&self) -> NormObjView<P> {
        //
        NormObjView {
            core: self.core.clone(),
            pool: self.pool.clone(),
        }
    }

    // Creates the total department and starts its single actor.
    fn new(core: RdbCore, pool: P, prom: M) -> Self {
        //
        let actor_core = core.clone();

        let actor_pool = pool.clone();

        let actor_desc = ObjActor::new(prom.clone(), move |task| {
            //
            let core = actor_core.clone();

            let pool = actor_pool.clone();

            async move { Self::dispatch(core, pool, task).await }
        });

        Self {
            core,
            pool,
            prom,
            actor_desc,
        }
    }
}

impl<P, M> NormObjDept<P, M>
where
    P: ObjPool,
    M: ObjProm,
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
            actor_desc: self.actor_desc.clone(),
        }
    }
}

impl_obj_dept! {
    dept: NormObjDept,
    view: NormObjView,
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

/// Builds the production `ObjDept` without exposing its actor-side adapter.
///
/// # Errors
///
/// Returns an error when the R2 object-storage configuration is unavailable.
pub fn new_obj_dept(
    core: RdbCore,
) -> anyhow::Result<NormObjDept<R2ObjPool, RdbObjProm>> {
    //
    let pool = R2ObjPool::from_env()?;

    let prom = RdbObjProm::new(core.clone());

    Ok(NormObjDept::new(core, pool, prom))
}

// Builds a stable invalid-key error at the concrete mapping boundary.
fn invalid_key(kind: &str) -> ObjDeptError {
    //
    ObjDeptError::Invalid {
        message: format!("invalid {} physical key", kind),
    }
}
