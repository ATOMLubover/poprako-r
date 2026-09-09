// Generates deferred object-promotion task operations.
mod defer;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Path, Result, Token, braced};

// Typed durable-task adapter declaration.
struct PromInput {
    //
    // Generated adapter type name.
    name: Ident,
    // Diesel task table path.
    table: Path,
}

impl Parse for PromInput {
    // Parses one durable-task adapter declaration.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let name = input.parse()?;

        let content;

        braced!(content in input);

        let field = content.parse::<Ident>()?;

        if field != "table" {
            return Err(syn::Error::new(field.span(), "expected `table`"));
        }

        content.parse::<Token![:]>()?;

        let table = content.parse()?;

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        if !content.is_empty() || !input.is_empty() {
            //
            return Err(
                input.error("unexpected ObjDeptProm declaration tokens")
            );
        }

        Ok(Self { name, table })
    }
}

/// Expands the typed Diesel `ObjDeptProm` adapter.
#[expect(
    clippy::too_many_lines,
    reason = "actor-side task transitions remain one auditable generated state machine"
)]
pub fn expand(input: TokenStream) -> Result<TokenStream> {
    //
    let PromInput { name, table } = syn::parse2(input)?;

    let module =
        format_ident!("{}_rdb_impl", to_snake_case(&name.to_string()),);

    let defer_module = defer::expand_module(&table);

    let defer_impl = defer::expand_impl(&name, &module);

    Ok(quote! {
        #[derive(Clone)]
        pub struct #name {
            core: ::poprako_rdb_core::RdbCore,
        }

        impl #name {
            //
            /// Constructs task storage from an injected database core.
            pub const fn new(core: ::poprako_rdb_core::RdbCore) -> Self {
                Self { core }
            }
        }

        mod #module {
            use super::#table;

            use ::diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use ::diesel::{OptionalExtension as _, SelectableHelper as _};
            use ::diesel_async::RunQueryDsl as _;

            use ::poprako_obj_dept::key::ObjKey;
            use ::poprako_obj_dept::model::task::ObjDeptPromTask;
            use ::poprako_obj_dept::rdb_impl::{diesel_err, rdb_err};
            use ::poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};
            use ::poprako_rdb_core::{RdbConn, RdbCore};

            const PENDING: &str = "obj_prom_status:pending";
            const PROCESSING: &str = "obj_prom_status:processing";
            const COMPLETED: &str = "obj_prom_status:completed";
            const OPERATOR: &str = "obj_prom_status:operator";
            const ORDINARY_GENERATION: i64 = 0;
            const RETRY_DELAY: ::time::Duration =
                ::time::Duration::minutes(5);
            const PROCESSING_TIMEOUT: ::time::Duration =
                ::time::Duration::minutes(3);

            #defer_module

            #[derive(::diesel::Queryable, ::diesel::Selectable)]
            #[diesel(table_name = #table)]
            #[diesel(check_for_backend(::diesel::pg::Pg))]
            struct FullRow {
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
                #[diesel(column_name = f_visible_at)]
                visible_at: ::time::OffsetDateTime,
                #[diesel(column_name = f_retried_count)]
                retried_count: i64,
                #[diesel(column_name = f_lease)]
                lease: i64,
                #[diesel(column_name = f_error)]
                error: Option<String>,

                #[diesel(column_name = f_created_at)]
                created_at: ::time::OffsetDateTime,
                #[diesel(column_name = f_updated_at)]
                updated_at: ::time::OffsetDateTime,
            }

            #[derive(::diesel::Queryable, ::diesel::Selectable)]
            #[diesel(table_name = #table)]
            #[diesel(check_for_backend(::diesel::pg::Pg))]
            struct TaskRow {
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
                #[diesel(column_name = f_retried_count)]
                retried_count: i64,
                #[diesel(column_name = f_lease)]
                lease: i64,
            }

            impl From<TaskRow> for ObjDeptPromTask {
                //
                fn from(row: TaskRow) -> Self {
                    //
                    Self {
                        id: row.id,
                        topic: row.topic,
                        oper: row.oper,
                        obj_id: row.obj_id,
                        ver: row.ver,
                        image: row.image,
                        gen_no: row.gen_no,
                        retried_count: row.retried_count,
                        lease: row.lease,
                    }
                }
            }

            pub async fn reset_tasks(core: &RdbCore) -> ObjDeptRest<usize> {
                let mut conn = core.get().await.map_err(rdb_err)?;
                let now = ::time::OffsetDateTime::now_utc();
                let before = now - PROCESSING_TIMEOUT;
                let invalid = ::diesel::update(
                    #table::table
                        .filter(#table::f_status.ne(PENDING))
                        .filter(#table::f_status.ne(PROCESSING))
                        .filter(#table::f_status.ne(COMPLETED))
                        .filter(#table::f_status.ne(OPERATOR)),
                )
                .set((
                    #table::f_status.eq(OPERATOR),
                    #table::f_error.eq(Some("invalid object task status")),
                    #table::f_updated_at.eq(now),
                ))
                .execute(&mut conn)
                .await
                .map_err(diesel_err)?;
                let overflow = ::diesel::update(
                    #table::table
                        .filter(#table::f_status.eq(PROCESSING))
                        .filter(#table::f_updated_at.le(before))
                        .filter(#table::f_lease.eq(i64::MAX)),
                )
                .set((
                    #table::f_status.eq(OPERATOR),
                    #table::f_error.eq(Some("object task lease overflow")),
                    #table::f_updated_at.eq(now),
                ))
                .execute(&mut conn)
                .await
                .map_err(diesel_err)?;
                let reset = ::diesel::update(
                    #table::table
                        .filter(#table::f_status.eq(PROCESSING))
                        .filter(#table::f_updated_at.le(before))
                        .filter(#table::f_lease.lt(i64::MAX)),
                )
                .set((
                    #table::f_status.eq(PENDING),
                    #table::f_lease.eq(#table::f_lease + 1),
                    #table::f_visible_at.eq(now),
                    #table::f_updated_at.eq(now),
                ))
                .execute(&mut conn)
                .await
                .map_err(diesel_err)?;

                Ok(invalid + overflow + reset)
            }

            pub async fn claim_task(
                core: &RdbCore,
            ) -> ObjDeptRest<Option<ObjDeptPromTask>> {
                let mut conn = core.get().await.map_err(rdb_err)?;
                let claim_candidate =
                    ::diesel::alias!(#table as claim_candidate);
                let candidate = claim_candidate
                    .filter(claim_candidate.field(#table::f_status).eq(PENDING))
                    .filter(
                        claim_candidate
                            .field(#table::f_visible_at)
                            .le(::time::OffsetDateTime::now_utc()),
                    )
                    .order_by((
                        claim_candidate.field(#table::f_visible_at).asc(),
                        claim_candidate.field(#table::f_created_at).asc(),
                        claim_candidate.field(#table::f_id).asc(),
                    ))
                    .select(claim_candidate.field(#table::f_id))
                    .for_update()
                    .skip_locked()
                    .limit(1);
                let lease_overflow = #table::f_lease.eq(i64::MAX);
                let status = ::diesel::dsl::case_when::<
                    _,
                    _,
                    ::diesel::sql_types::Text,
                >(lease_overflow, OPERATOR)
                .otherwise(PROCESSING);
                let lease = ::diesel::dsl::case_when::<
                    _,
                    _,
                    ::diesel::sql_types::BigInt,
                >(lease_overflow, #table::f_lease)
                .otherwise(#table::f_lease + 1);
                let error = ::diesel::dsl::case_when::<
                    _,
                    _,
                    ::diesel::sql_types::Nullable<::diesel::sql_types::Text>,
                >(lease_overflow, Some("object task lease overflow"))
                .otherwise(#table::f_error);
                let row = ::diesel::update(
                    #table::table.filter(#table::f_id.eq_any(candidate)),
                )
                .set((
                    #table::f_status.eq(status),
                    #table::f_lease.eq(lease),
                    #table::f_error.eq(error),
                    #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                ))
                .returning((TaskRow::as_returning(), #table::f_status))
                .get_result::<(TaskRow, String)>(&mut conn)
                .await
                .optional()
                .map_err(diesel_err)?;
                let Some((row, status)) = row else {
                    return Ok(None);
                };

                match status.as_str() {
                    OPERATOR => Ok(None),
                    PROCESSING => Ok(Some(row.into())),
                    _ => Err(ObjDeptError::Unrecoverable {
                        message: "object task claim returned invalid status".into(),
                    }),
                }
            }

            pub async fn complete_task(
                core: &RdbCore,
                task: &ObjDeptPromTask,
            ) -> ObjDeptRest<usize> {
                let mut conn = core.get().await.map_err(rdb_err)?;

                ::diesel::update(
                    #table::table
                        .filter(#table::f_id.eq(&task.id))
                        .filter(#table::f_status.eq(PROCESSING))
                        .filter(#table::f_lease.eq(task.lease)),
                )
                .set((
                    #table::f_status.eq(COMPLETED),
                    #table::f_error.eq(None::<String>),
                    #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                ))
                .execute(&mut conn)
                .await
                .map_err(diesel_err)
            }

            pub async fn retry_task(
                core: &RdbCore,
                task: &ObjDeptPromTask,
                message: &str,
            ) -> ObjDeptRest<usize> {
                let mut conn = core.get().await.map_err(rdb_err)?;

                ::diesel::update(
                    #table::table
                        .filter(#table::f_id.eq(&task.id))
                        .filter(#table::f_status.eq(PROCESSING))
                        .filter(#table::f_lease.eq(task.lease)),
                )
                .set((
                    #table::f_status.eq(PENDING),
                    #table::f_retried_count
                        .eq(task.retried_count.saturating_add(1)),
                    #table::f_visible_at.eq(
                        ::time::OffsetDateTime::now_utc() + RETRY_DELAY,
                    ),
                    #table::f_error.eq(Some(message)),
                    #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                ))
                .execute(&mut conn)
                .await
                .map_err(diesel_err)
            }

            pub async fn mark_task_operator(
                core: &RdbCore,
                task: &ObjDeptPromTask,
                message: &str,
            ) -> ObjDeptRest<usize> {
                let mut conn = core.get().await.map_err(rdb_err)?;

                ::diesel::update(
                    #table::table
                        .filter(#table::f_id.eq(&task.id))
                        .filter(#table::f_status.eq(PROCESSING))
                        .filter(#table::f_lease.eq(task.lease)),
                )
                .set((
                    #table::f_status.eq(OPERATOR),
                    #table::f_error.eq(Some(message)),
                    #table::f_updated_at.eq(::time::OffsetDateTime::now_utc()),
                ))
                .execute(&mut conn)
                .await
                .map_err(diesel_err)
            }

            #[allow(dead_code)]
            fn assert_full_schema(row: FullRow) {
                //
                let FullRow {
                    //
                    id,
                    topic,
                    oper,
                    obj_id,
                    ver,
                    image,
                    gen_no: gen_value,
                    status,
                    visible_at,
                    retried_count,
                    lease,
                    error,
                    created_at,
                    updated_at,
                } = row;

                drop((
                    id,
                    topic,
                    oper,
                    obj_id,
                    ver,
                    image,
                    gen_value,
                    status,
                    visible_at,
                    retried_count,
                    lease,
                    error,
                    created_at,
                    updated_at,
                ));
            }
        }

        impl ::poprako_obj_dept::prom::ObjDeptProm for #name {
            async fn reset_tasks(
                &self,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
                #module::reset_tasks(&self.core).await
            }

            async fn claim_task(
                &self,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                Option<::poprako_obj_dept::model::task::ObjDeptPromTask>,
            > {
                #module::claim_task(&self.core).await
            }

            async fn complete_task(
                &self,
                task: &::poprako_obj_dept::model::task::ObjDeptPromTask,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
                #module::complete_task(&self.core, task).await
            }

            async fn retry_task<'a>(
                &'a self,
                task: &'a ::poprako_obj_dept::model::task::ObjDeptPromTask,
                message: &'a str,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
                #module::retry_task(&self.core, task, message).await
            }

            async fn mark_task_operator<'a>(
                &'a self,
                task: &'a ::poprako_obj_dept::model::task::ObjDeptPromTask,
                message: &'a str,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<usize> {
                #module::mark_task_operator(&self.core, task, message).await
            }
        }

        #defer_impl
    })
}

// Converts a generated type name into its helper-module name.
fn to_snake_case(name: &str) -> String {
    //
    let mut snake_case = String::with_capacity(name.len());

    let mut chars = name.chars().peekable();

    let mut prev = None;

    while let Some(current) = chars.next() {
        //
        let next = chars.peek().copied();

        let follows_lowercase = prev.is_some_and(|character: char| {
            character.is_lowercase() || character.is_ascii_digit()
        });

        let is_word_boundary = current.is_uppercase()
            && !snake_case.is_empty()
            && (follows_lowercase || next.is_some_and(char::is_lowercase));

        if is_word_boundary {
            snake_case.push('_');
        }

        snake_case.extend(current.to_lowercase());

        prev = Some(current);
    }

    snake_case
}
