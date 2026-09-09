//! Physical object-storage contract used by ObjDept.

use std::collections::HashMap;
use std::future::Future;

use futures_util::future::try_join_all;

use crate::model::meta::ObjMeta;
use crate::model::slot::ObjDeptPoolSlot;
use crate::model::url::{ObjUrlSpec, ObjUrls};
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Read-only physical object operations.
pub trait ObjDeptPoolView {
    /// Generates public or signed read URLs for one physical key and profile.
    fn gen_urls(
        &self,
        key: &str,
        spec: ObjUrlSpec,
    ) -> impl Future<Output = ObjDeptRest<ObjUrls>> + Send;

    /// Checks whether one physical key exists.
    fn has(&self, key: &str) -> impl Future<Output = ObjDeptRest<bool>> + Send;
}

/// Complete storage-neutral physical object operations.
pub trait ObjDeptPool: ObjDeptPoolView {
    /// Generates a write capability for one physical key.
    fn gen_slot(
        &self,
        key: &str,
        content_type: &str,
        byte_len: u64,
    ) -> impl Future<Output = ObjDeptRest<ObjDeptPoolSlot>> + Send;

    /// Deletes one physical key idempotently.
    fn del(&self, key: &str) -> impl Future<Output = ObjDeptRest<()>> + Send;
}

/// Rejects an object URL specification that selects no rendition.
#[doc(hidden)]
pub fn ensure_url_spec(spec: ObjUrlSpec) -> ObjDeptRest<()> {
    //
    if spec.is_empty() {
        //
        return Err(ObjDeptError::Invalid {
            message: "at least one object URL must be selected".into(),
        });
    }

    Ok(())
}

/// Resolves uploaded metadata through a bounded number of pool futures.
#[doc(hidden)]
pub async fn gen_urls_bounded<P, S>(
    pool: &P,
    spec: ObjUrlSpec,
    metas: &HashMap<String, ObjMeta, S>,
) -> ObjDeptRest<HashMap<String, ObjUrls>>
where
    P: ObjDeptPoolView + Sync,
    S: std::hash::BuildHasher + Sync,
{
    // Maximum pool requests resolved concurrently in one batch.
    const CONCURRENCY: usize = 20;

    ensure_url_spec(spec)?;

    let mut uploaded = metas
        .iter()
        .filter(|(_, meta)| meta.is_avail)
        .collect::<Vec<_>>();

    uploaded.sort_unstable_by_key(|(id, _)| id.as_str());

    let mut urls = Vec::with_capacity(uploaded.len());

    for chunk in uploaded.chunks(CONCURRENCY) {
        //
        let futures = chunk.iter().map(|(id, meta)| async move {
            //
            let obj_urls = pool.gen_urls(&meta.key.image, spec).await?;

            Ok(((*id).clone(), obj_urls))
        });

        urls.extend(try_join_all(futures).await?);
    }

    Ok(urls.into_iter().collect())
}
