use std::collections::{HashMap, HashSet};

use poprako_orchestra::{OperRun as _, Run};

use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, BaseRest};

/// Pinned chapters loaded for an explicit comic range.
///
/// The queried range records that a missing map entry means no pinned chapter
/// exists, rather than that the comic has not been queried yet.
pub struct PinnedChapterSnapshot {
    //
    /// Comic identifiers included in the pinned-chapter query.
    queried_comic_ids: HashSet<String>,

    /// Pinned chapters keyed by comic identifier.
    infos_by_comic_id: HashMap<String, ChapterInfo>,
}

impl PinnedChapterSnapshot {
    /// Loads pinned chapters and records every comic identifier queried.
    pub async fn load_from_comics<R>(
        repo: &R,
        comic_ids: &[&str],
    ) -> BaseRest<Self>
    where
        R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError> + Sync,
    {
        let mut queried_comic_ids = comic_ids
            .iter()
            .map(|comic_id| (*comic_id).to_owned())
            .collect::<Vec<_>>();

        queried_comic_ids.sort_unstable();

        queried_comic_ids.dedup();

        let query_comic_ids = queried_comic_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();

        let chapter_infos = match query_comic_ids.as_slice() {
            //
            [] => Vec::new(),

            _ => {
                //
                ListPinnedChapterInfos {
                    comic_ids: &query_comic_ids,
                }
                .run_on(repo)
                .await?
            }
        };

        let infos_by_comic_id = chapter_infos
            .into_iter()
            .map(|chapter_info| (chapter_info.comic_id.clone(), chapter_info))
            .collect();

        Ok(Self {
            queried_comic_ids: queried_comic_ids.into_iter().collect(),
            infos_by_comic_id,
        })
    }

    /// Returns the loaded pinned chapters keyed by comic identifier.
    pub const fn infos_by_comic_id(&self) -> &HashMap<String, ChapterInfo> {
        &self.infos_by_comic_id
    }

    /// Consumes the snapshot into pinned chapters keyed by comic identifier.
    pub fn into_infos_by_comic_id(self) -> HashMap<String, ChapterInfo> {
        self.infos_by_comic_id
    }

    // Returns a loaded chapter, including the known-absent case as `None`.
    fn get(&self, comic_id: &str) -> Option<&ChapterInfo> {
        self.infos_by_comic_id.get(comic_id)
    }

    // Checks whether a comic was included in this snapshot's query range.
    fn has_queried_comic(&self, comic_id: &str) -> bool {
        self.queried_comic_ids.contains(comic_id)
    }
}

/// Loads page models needed by use-case orchestration.
pub struct PageLoader;

impl PageLoader {
    /// Loads first-page identifiers keyed by their owning comic identifiers.
    ///
    /// Object availability remains the object department's responsibility;
    /// this loader only resolves the domain relationship used by cover fallback.
    pub async fn load_ids_from_comics<R>(
        repo: &R,
        comic_ids: &[&str],
        pinned_chapter_snapshot: Option<&PinnedChapterSnapshot>,
    ) -> BaseRest<HashMap<String, String>>
    where
        R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
            + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
            + Sync,
    {
        let mut comic_ids = comic_ids.to_vec();

        comic_ids.sort_unstable();

        comic_ids.dedup();

        if comic_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut comic_ids_by_chapter_id = HashMap::new();

        let mut unloaded_comic_ids = Vec::new();

        match pinned_chapter_snapshot {
            //
            Some(pinned_chapter_snapshot) => {
                //
                for comic_id in &comic_ids {
                    // A queried comic without an entry has no pinned chapter.
                    if pinned_chapter_snapshot.has_queried_comic(comic_id) {
                        //
                        if let Some(chapter_info) =
                            pinned_chapter_snapshot.get(comic_id)
                        {
                            //
                            comic_ids_by_chapter_id.insert(
                                chapter_info.id.clone(),
                                chapter_info.comic_id.clone(),
                            );
                        }

                        continue;
                    }

                    unloaded_comic_ids.push(*comic_id);
                }
            }

            None => unloaded_comic_ids.extend(comic_ids),
        }

        let unloaded_pinned_chapter_snapshot =
            PinnedChapterSnapshot::load_from_comics(repo, &unloaded_comic_ids)
                .await?;

        comic_ids_by_chapter_id.extend(
            unloaded_pinned_chapter_snapshot
                .infos_by_comic_id()
                .values()
                .map(|chapter_info| {
                    (chapter_info.id.clone(), chapter_info.comic_id.clone())
                }),
        );

        let chapter_ids = comic_ids_by_chapter_id
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();

        let page_infos = ListFirstPageInfos {
            chapter_ids: &chapter_ids,
        }
        .run_on(repo)
        .await?;

        let page_ids = page_infos
            .into_iter()
            .filter_map(|page_info| {
                //
                comic_ids_by_chapter_id
                    .get(&page_info.chapter_id)
                    .cloned()
                    .map(|comic_id| (comic_id, page_info.id))
            })
            .collect();

        Ok(page_ids)
    }
}
