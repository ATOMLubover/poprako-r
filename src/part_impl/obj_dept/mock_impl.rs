//! In-memory ObjDept operations used by server tests.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use time::{Duration, OffsetDateTime};
use url::Url;

use poprako_obj_dept::key::{KeyMap, ObjKey};
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::slot::ObjSlot;
use poprako_obj_dept::model::task::ObjTask;
use poprako_obj_dept::model::url::{ObjUrlSpec, ObjUrls};
use poprako_obj_dept::oper::{GenObjSlot, GenObjSlots};
use poprako_obj_dept::pool::ensure_url_spec;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};

use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};

pub fn gen_urls(
    meta: Option<&ObjMeta>,
    spec: ObjUrlSpec,
    thumbnail_enabled: bool,
) -> ObjDeptRest<Option<ObjUrls>> {
    ensure_url_spec(spec)?;

    let Some(meta) = meta else {
        return Ok(None);
    };

    if !meta.is_avail {
        return Ok(None);
    }

    let key = &meta.key.image;

    let origin_url = match spec.includes_origin() {
        true => Some(parse_mock_url(key)?),
        false => None,
    };

    let optimized_url = match spec.includes_optimized() {
        true => Some(parse_mock_url(&format!("optimized/{}", key))?),
        false => None,
    };

    let thumbnail_url = match (spec.includes_thumbnail(), thumbnail_enabled) {
        (true, true) => Some(parse_mock_url(&format!("thumbnail/{}", key))?),
        (true, false) | (false, _) => None,
    };

    Ok(Some(ObjUrls {
        origin_url,
        optimized_url,
        thumbnail_url,
    }))
}

// Parses one deterministic object URL for tests.
fn parse_mock_url(path: &str) -> ObjDeptRest<Url> {
    Url::parse(&format!("https://obj.test/{}", path)).map_err(|source| {
        ObjDeptError::Unrecoverable {
            message: source.to_string(),
        }
    })
}

pub fn gen_slot<K>(
    objs: &mut HashMap<String, MockObjRecord>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    topic: &'static str,
    oper: &GenObjSlot<'_, K>,
) -> ObjDeptRest<Option<ObjSlot>>
where
    K: KeyMap<Img = String>,
{
    let id = K::id(&oper.spec.dom);

    let matching_meta = objs
        .get(id)
        .and_then(|record| record.meta.as_ref())
        .filter(|meta| {
            meta.hash == oper.spec.hash && meta.ext == K::ext(&oper.spec.dom)
        });

    if matching_meta.is_some_and(|meta| meta.is_avail) {
        return Ok(None);
    }

    let reused_key = matching_meta.map(|meta| meta.key.clone());

    let (key, previous) = match reused_key {
        Some(key) => (key, None),
        None => {
            let ver = objs.get(id).map_or(Ok(1), |previous| {
                previous.version.checked_add(1).ok_or_else(|| {
                    ObjDeptError::Unrecoverable {
                        message: "object ver overflow".into(),
                    }
                })
            })?;

            let key = ObjKey {
                id: id.to_owned(),
                ver,
                image: K::forward(&oper.spec.dom, ver),
            };

            let meta = ObjMeta {
                key: key.clone(),
                is_avail: false,
                hash: oper.spec.hash.to_vec(),
                ext: K::ext(&oper.spec.dom).to_owned(),
            };

            let previous = objs.insert(
                id.to_owned(),
                MockObjRecord {
                    version: ver,
                    meta: Some(meta),
                },
            );

            (key, previous)
        }
    };

    if let Some(previous_key) =
        previous.and_then(|record| record.meta.map(|meta| meta.key))
    {
        tasks.push((topic, ObjTask::Delete { key: previous_key }));
    }

    tasks.push((topic, ObjTask::Check { key: key.clone() }));

    let url = Url::parse(&format!("https://obj.test/write/{}", key.image))
        .map_err(|source| ObjDeptError::Unrecoverable {
            message: source.to_string(),
        })?;

    Ok(Some(ObjSlot {
        key,
        url,
        headers: Default::default(),
        expires_at: OffsetDateTime::now_utc() + Duration::minutes(5),
    }))
}

pub fn gen_slots<K>(
    objs: &mut HashMap<String, MockObjRecord>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    topic: &'static str,
    oper: &GenObjSlots<'_, K>,
) -> ObjDeptRest<HashMap<String, ObjSlot>>
where
    K: KeyMap<Img = String>,
{
    let mut ids = oper
        .specs
        .iter()
        .map(|spec| K::id(&spec.dom))
        .collect::<Vec<_>>();

    ids.sort_unstable();

    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ObjDeptError::Invalid {
            message: "duplicate object slot id".into(),
        });
    }

    oper.specs
        .iter()
        .map(|spec| {
            let single_oper = GenObjSlot::<K>::new(spec);
            let slot = gen_slot(objs, tasks, topic, &single_oper)?;

            Ok(slot.map(|slot| (K::id(&spec.dom).to_owned(), slot)))
        })
        .filter_map(|result| result.transpose())
        .collect()
}

pub fn clear_objs(
    objs: &mut HashMap<String, MockObjRecord>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    topic: &'static str,
    ids: &[String],
) {
    for id in ids {
        let Some(record) = objs.get_mut(id) else {
            continue;
        };

        if let Some(meta) = record.meta.take() {
            tasks.push((topic, ObjTask::Delete { key: meta.key }));
        }
    }
}

pub fn delete_objs(
    objs: &mut HashMap<String, MockObjRecord>,
    tasks: &mut Vec<(&'static str, ObjTask)>,
    topic: &'static str,
    ids: &[String],
) {
    for id in ids {
        let Some(record) = objs.remove(id) else {
            continue;
        };

        if let Some(meta) = record.meta {
            tasks.push((topic, ObjTask::Delete { key: meta.key }));
        }
    }
}

#[macro_export]
macro_rules! implement_mock_obj_dept {
    ($obj:ty, $topic:literal) => {
        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<bool> {
                let mut state = self.state.lock().unwrap();
                let Some(record) = state
                    .objs
                    .get_mut($topic)
                    .and_then(|objs| objs.get_mut(&oper.key.id))
                else {
                    return Ok(false);
                };

                if record.version != oper.key.ver {
                    return Ok(false);
                }

                let Some(meta) = record.meta.as_mut() else {
                    return Ok(false);
                };

                meta.is_avail = true;

                Ok(true)
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::MarkObjUploaded<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<bool> {
                let state = &mut context.state;
                let Some(record) = state
                    .objs
                    .get_mut($topic)
                    .and_then(|objs| objs.get_mut(&oper.key.id))
                else {
                    return Ok(false);
                };

                if record.version != oper.key.ver {
                    return Ok(false);
                }

                let Some(meta) = record.meta.as_mut() else {
                    return Ok(false);
                };

                meta.is_avail = true;

                Ok(true)
            }
        }

        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::ListObjMetas<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::ListObjMetas<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::meta::ObjMeta,
                >,
            > {
                let state = self.state.lock().unwrap();

                Ok(oper
                    .ids
                    .iter()
                    .copied()
                    .filter_map(|id| {
                        state
                            .objs
                            .get($topic)
                            .and_then(|objs| objs.get(id))
                            .and_then(|record| record.meta.clone())
                            .map(|obj_meta| (id.to_owned(), obj_meta))
                    })
                    .collect())
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::ListObjMetas<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::ListObjMetas<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::meta::ObjMeta,
                >,
            > {
                Ok(oper
                    .ids
                    .iter()
                    .copied()
                    .filter_map(|id| {
                        context
                            .state
                            .objs
                            .get($topic)
                            .and_then(|objs| objs.get(id))
                            .and_then(|record| record.meta.clone())
                            .map(|obj_meta| (id.to_owned(), obj_meta))
                    })
                    .collect())
            }
        }

        impl<'a>
            ::poprako_orchestra::Run<
                ::poprako_obj_dept::oper::GenObjUrls<'a, $obj>,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn run(
                &self,
                oper: &::poprako_obj_dept::oper::GenObjUrls<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    ::std::string::String,
                    ::poprako_obj_dept::model::url::ObjUrls,
                >,
            > {
                ::poprako_obj_dept::pool::ensure_url_spec(oper.spec)?;

                let mut urls = ::std::collections::HashMap::new();
                let thumbnail_enabled =
                    !self.flags.lock().unwrap().obj_thumbnail_disabled;

                for (id, obj_meta) in oper.metas {
                    let Some(url) =
                        $crate::part_impl::obj_dept::mock_impl::gen_urls(
                            Some(obj_meta),
                            oper.spec,
                            thumbnail_enabled,
                        )?
                    else {
                        continue;
                    };

                    urls.insert(id.clone(), url);
                }

                Ok(urls)
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::GenObjSlots<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::GenObjSlots<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::std::collections::HashMap<
                    String,
                    ::poprako_obj_dept::model::slot::ObjSlot,
                >,
            > {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::mock_impl::gen_slots(
                    objs, obj_tasks, $topic, oper,
                )
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::GenObjSlot<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::GenObjSlot<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                Option<::poprako_obj_dept::model::slot::ObjSlot>,
            > {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::mock_impl::gen_slot(
                    objs, obj_tasks, $topic, oper,
                )
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::ClearObjs<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::ClearObjs<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::mock_impl::clear_objs(
                    objs, obj_tasks, $topic, oper.ids,
                );

                Ok(())
            }
        }

        impl<'a>
            ::poprako_orchestra::Step<
                ::poprako_obj_dept::oper::DeleteObjs<'a, $obj>,
                $crate::part_impl::repo::mock_impl::MockContext,
            > for $crate::part_impl::repo::mock_impl::Mock
        {
            type Level = $crate::part::nucl::ReptRead;
            type Error = ::poprako_obj_dept::rest::ObjDeptError;

            async fn step(
                &self,
                context: &mut $crate::part_impl::repo::mock_impl::MockContext,
                oper: &::poprako_obj_dept::oper::DeleteObjs<'a, $obj>,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                if context.obj_delete_failure {
                    return Err(
                        ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                            message: "mock object deletion failed".into(),
                        },
                    );
                }

                let $crate::part_impl::repo::mock_impl::MockState {
                    objs,
                    obj_tasks,
                    ..
                } = &mut context.state;

                let objs = objs.entry($topic).or_default();

                $crate::part_impl::obj_dept::mock_impl::delete_objs(
                    objs, obj_tasks, $topic, oper.ids,
                );

                Ok(())
            }
        }
    };
}
