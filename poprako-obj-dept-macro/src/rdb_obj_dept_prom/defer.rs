use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path};

/// Generates transaction-side single and batch task creation.
#[expect(
    clippy::too_many_lines,
    reason = "task identity insertion and validation form one atomic generated contract"
)]
pub fn expand_module(table: &Path) -> TokenStream {
    //
    quote! {
        #[derive(::diesel::Queryable, ::diesel::Selectable)]
        #[diesel(table_name = #table)]
        #[diesel(check_for_backend(::diesel::pg::Pg))]
        struct TaskIdentityRow {
            #[diesel(column_name = f_id)]
            id: String,

            #[diesel(column_name = f_topic)]
            topic: String,
            #[diesel(column_name = f_oper)]
            oper: String,

            #[diesel(column_name = f_obj_id)]
            obj_id: String,
            #[diesel(column_name = f_version)]
            ver: i64,
            #[diesel(column_name = f_key)]
            image: String,

            #[diesel(column_name = f_generation)]
            gen_no: i64,
            #[diesel(column_name = f_status)]
            status: String,
        }

        #[derive(::diesel::Insertable)]
        #[diesel(table_name = #table)]
        struct TaskInsertRow<'a> {
            #[diesel(column_name = f_id)]
            id: &'a str,

            #[diesel(column_name = f_topic)]
            topic: &'a str,
            #[diesel(column_name = f_oper)]
            oper: &'a str,

            #[diesel(column_name = f_obj_id)]
            obj_id: &'a str,
            #[diesel(column_name = f_version)]
            ver: i64,
            #[diesel(column_name = f_key)]
            image: &'a str,

            #[diesel(column_name = f_generation)]
            gen_no: i64,
            #[diesel(column_name = f_status)]
            status: &'a str,
            #[diesel(column_name = f_visible_at)]
            visible_at: ::time::OffsetDateTime,
        }

        pub struct PreparedTask {
            pub id: String,

            oper: &'static str,

            key: ObjKey,

            visible_at: ::time::OffsetDateTime,
        }

        impl PreparedTask {
            //
            pub fn new(
                topic: &str,
                oper: &'static str,
                key: ObjKey,
                visible_at: ::time::OffsetDateTime,
            ) -> Self {
                //
                let id = ::poprako_obj_dept::model::task::obj_task_id(
                    topic,
                    oper,
                    &key,
                    ORDINARY_GENERATION,
                );

                Self {
                    id,
                    oper,
                    key,
                    visible_at,
                }
            }
        }

        pub async fn defer_tasks(
            conn: &mut RdbConn,
            topic: &str,
            tasks: &[PreparedTask],
        ) -> ObjDeptRest<()> {
            use ::std::collections::HashMap;

            if tasks.is_empty() {
                return Ok(());
            }

            let rows = tasks
                .iter()
                .map(|task| TaskInsertRow {
                    id: &task.id,
                    topic,
                    oper: task.oper,
                    obj_id: &task.key.id,
                    ver: i64::from(task.key.ver),
                    image: &task.key.image,
                    gen_no: ORDINARY_GENERATION,
                    status: PENDING,
                    visible_at: task.visible_at,
                })
                .collect::<Vec<_>>();

            ::diesel::insert_into(#table::table)
                .values(&rows)
                .on_conflict_do_nothing()
                .execute(conn)
                .await
                .map_err(diesel_err)?;

            let ids = tasks.iter().map(|task| &task.id).collect::<Vec<_>>();
            let rows = #table::table
                .filter(#table::f_id.eq_any(ids))
                .select(TaskIdentityRow::as_select())
                .load::<TaskIdentityRow>(conn)
                .await
                .map_err(diesel_err)?;
            let rows = rows
                .into_iter()
                .map(|row| {
                    let TaskIdentityRow {
                        id,
                        topic,
                        oper,
                        obj_id,
                        ver,
                        image,
                        gen_no: gen_value,
                        status,
                    } = row;

                    (
                        id,
                        (
                            topic,
                            oper,
                            obj_id,
                            ver,
                            image,
                            gen_value,
                            status,
                        ),
                    )
                })
                .collect::<HashMap<_, _>>();

            if rows.len() != tasks.len() {
                //
                return Err(ObjDeptError::Unrecoverable {
                    message: "object task batch identity is incomplete".into(),
                });
            }

            for task in tasks {
                //
                let Some(row) = rows.get(&task.id) else {
                    //
                    return Err(ObjDeptError::Unrecoverable {
                        message: "object task batch identity is incomplete".into(),
                    });
                };

                let (
                    row_topic,
                    row_oper,
                    row_obj_id,
                    row_ver,
                    row_image,
                    row_gen,
                    row_status,
                ) = row;

                let is_same_identity = row_topic == topic
                    && row_oper == task.oper
                    && row_obj_id == &task.key.id
                    && *row_ver == i64::from(task.key.ver)
                    && row_image == &task.key.image
                    && *row_gen == ORDINARY_GENERATION;

                if !is_same_identity {
                    //
                    return Err(ObjDeptError::Unrecoverable {
                        message: "object task identity conflict".into(),
                    });
                }

                match row_status.as_str() {
                    //
                    PENDING | PROCESSING | COMPLETED => {}

                    OPERATOR => {
                        //
                        return Err(ObjDeptError::Conflict {
                            message: "object task requires operator repair".into(),
                        });
                    }

                    _ => {
                        //
                        return Err(ObjDeptError::Unrecoverable {
                            message: "invalid object task status".into(),
                        });
                    }
                }
            }

            Ok(())
        }
    }
}

/// Generates the public transaction-side adapter implementation.
pub fn expand_impl(name: &Ident, module: &Ident) -> TokenStream {
    //
    quote! {
        impl<L> ::poprako_obj_dept::prom::ObjDeptPromDefer<
            ::poprako_rdb_core::RdbContext<L>,
        > for #name
        where
            L: ::poprako_orchestra::Level + Send,
        {
            async fn defer_check<'a>(
                &'a self,
                context: &'a mut ::poprako_rdb_core::RdbContext<L>,
                topic: &'a str,
                key: &'a ::poprako_obj_dept::key::ObjKey,
                expires_at: ::time::OffsetDateTime,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                //
                let check = ::poprako_obj_dept::prom::ObjDeptPromCheck::new(
                    key.clone(),
                    expires_at,
                );

                self.defer_checks(context, topic, &[check]).await
            }

            async fn defer_delete<'a>(
                &'a self,
                context: &'a mut ::poprako_rdb_core::RdbContext<L>,
                topic: &'a str,
                key: &'a ::poprako_obj_dept::key::ObjKey,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                //
                self.defer_deletes(context, topic, ::std::slice::from_ref(key))
                    .await
            }

            async fn defer_checks<'a>(
                &'a self,
                context: &'a mut ::poprako_rdb_core::RdbContext<L>,
                topic: &'a str,
                checks: &'a [::poprako_obj_dept::prom::ObjDeptPromCheck],
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                let mut tasks = checks
                    .iter()
                    .map(|check| {
                        let visible_at = check
                            .expires_at()
                            .checked_add(::time::Duration::minutes(1))
                            .ok_or_else(|| {
                                ::poprako_obj_dept::rest::ObjDeptError::Unrecoverable {
                                    message: "object check visibility overflow".into(),
                                }
                            })?;

                        Ok(#module::PreparedTask::new(
                            topic,
                            ::poprako_obj_dept::model::task::CHECK,
                            check.key().clone(),
                            visible_at,
                        ))
                    })
                    .collect::<::poprako_obj_dept::rest::ObjDeptRest<Vec<_>>>()?;

                tasks.sort_unstable_by(|left, right| left.id.cmp(&right.id));
                tasks.dedup_by(|left, right| left.id == right.id);

                #module::defer_tasks(context.conn(), topic, &tasks).await
            }

            async fn defer_deletes<'a>(
                &'a self,
                context: &'a mut ::poprako_rdb_core::RdbContext<L>,
                topic: &'a str,
                keys: &'a [::poprako_obj_dept::key::ObjKey],
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<()> {
                let now = ::time::OffsetDateTime::now_utc();
                let mut tasks = keys
                    .iter()
                    .map(|key| {
                        #module::PreparedTask::new(
                            topic,
                            ::poprako_obj_dept::model::task::DELETE,
                            key.clone(),
                            now,
                        )
                    })
                    .collect::<Vec<_>>();

                tasks.sort_unstable_by(|left, right| left.id.cmp(&right.id));
                tasks.dedup_by(|left, right| left.id == right.id);

                #module::defer_tasks(context.conn(), topic, &tasks).await
            }
        }
    }
}
