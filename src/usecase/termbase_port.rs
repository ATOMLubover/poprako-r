//! Native terminology-base import and export use cases.

#[cfg(test)]
// Unit tests for native terminology-base import, merge, and export.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::comic::ComicComplex;
use crate::complex::termbase::{TermbaseComplex, TermbasePermComplex};
use crate::data::instr::termbase_port::ImportTermbaseInstr;
use crate::data::val::termbase_port::{ExportTermbaseVal, ImportTermbaseVal};
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::termbase::{TermbaseImport, TermbaseRepl};
use crate::part::nucl::ReptRead;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::GetComicInfoExcluded;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::LockTeam;
use crate::part::repo::oper::term::{ListTermInfos, UpsertTerms};
use crate::part::repo::oper::termbase::{
    CreateTermbase, GetTermbaseInfo, ListTermbaseInfosExcluded, UpdateTermbase,
    UpdateTermbaseTermCount,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::termbase::TermbaseScope;

/// Exports one terminology base as a native portable document.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn export<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<ExportTermbaseVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TermbaseRepo<C>
        + TermRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
{
    let export_termbase_val = nucl
        .coord(async move |context| {
            //
            let termbase_info =
                GetTermbaseInfo { id: &id }.step_on(repo, context).await?;

            let member_info = MemberLoader::load_info_from_termbase(
                repo,
                LoadMode::Step { context },
                &token.user_id,
                &termbase_info,
            )
            .await?;

            TermbasePermComplex::ensure_user_can_read(
                &member_info,
                &termbase_info,
            )?;

            let term_infos = ListTermInfos::Termbase {
                termbase_id: &termbase_info.id,
            }
            .step_on(repo, context)
            .await?;

            accept(ExportTermbaseVal::from_models(termbase_info, term_infos))
        })
        .await?;

    accept(export_termbase_val)
}

/// Imports a native portable document into a team or comic scope.
#[instrument(
    level = "info",
    skip(nucl, repo, token),
    fields(actor_user_id = %token.user_id),
)]
pub async fn import<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    scope: TermbaseScope,
    force_merge: bool,
    instr: ImportTermbaseInstr,
) -> BaseRest<ImportTermbaseVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TeamRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + TermbaseRepo<C>
        + TermRepo<C>
        + Send
        + Sync,
{
    let termbase_import = TermbaseComplex::normalize_import(instr.into())?;

    let import_termbase_val = nucl
        .coord(async move |context| {
            //
            let team_id = resolve_import_team_id(repo, context, &scope).await?;

            ensure_import_member(repo, context, &token, &team_id).await?;

            let termbase_infos =
                list_import_targets(repo, context, &scope).await?;

            let existing_termbase_info = TermbaseComplex::find_import_target(
                termbase_infos,
                &termbase_import.name,
            );

            let Some(termbase_info) = existing_termbase_info else {
                //
                let (team_id, comic_id) = match &scope {
                    //
                    TermbaseScope::Team { team_id } => {
                        (Some(team_id.clone()), None)
                    }

                    TermbaseScope::Comic { comic_id } => {
                        (None, Some(comic_id.clone()))
                    }
                };

                let termbase_entry = TermbaseComplex::build_entry(
                    team_id,
                    comic_id,
                    termbase_import.name.clone(),
                    termbase_import.description.clone(),
                    token.user_id.clone(),
                )?;

                let termbase_info = CreateTermbase {
                    entry: &termbase_entry,
                }
                .step_on(repo, context)
                .await?;

                return apply_import(
                    repo,
                    context,
                    &token,
                    termbase_info,
                    termbase_import,
                    true,
                )
                .await;
            };

            if !force_merge {
                return Err(already_exists_err(&termbase_import.name));
            }

            apply_import(
                repo,
                context,
                &token,
                termbase_info,
                termbase_import,
                false,
            )
            .await
        })
        .await?;

    accept(import_termbase_val)
}

// Lock the import scope and resolve its owning team.
async fn resolve_import_team_id<C, R>(
    repo: &R,
    context: &mut C,
    scope: &TermbaseScope,
) -> BaseRest<String>
where
    C: Context,
    R: TeamRepo<C> + ComicRepo<C> + WorksetRepo<C> + Sync,
{
    match scope {
        //
        TermbaseScope::Team { team_id } => {
            //
            LockTeam { id: team_id }.step_on(repo, context).await?;

            accept(team_id.clone())
        }

        TermbaseScope::Comic { comic_id } => {
            //
            let comic_info = GetComicInfoExcluded {
                id: comic_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ComicComplex::ensure_comic_writable(&comic_info)?;

            let workset_info = GetWorksetInfo {
                id: &comic_info.workset_id,
            }
            .step_on(repo, context)
            .await?;

            accept(workset_info.team_id)
        }
    }
}

// Require import permission through membership in the resolved team.
async fn ensure_import_member<C, R>(
    repo: &R,
    context: &mut C,
    token: &UserToken,
    team_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: MemberRepo<C> + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id,
    }
    .step_on(repo, context)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-translator-or-proofreader-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            team_id = %team_id,
            user_id = %token.user_id,
            "expected error: termbase importer membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    TermbasePermComplex::ensure_user_can_write_team(&member_info)
}

// List existing terminology bases within the selected import scope.
async fn list_import_targets<C, R>(
    repo: &R,
    context: &mut C,
    scope: &TermbaseScope,
) -> BaseRest<Vec<TermbaseInfo>>
where
    C: Context,
    R: TermbaseRepo<C> + Sync,
{
    match scope {
        //
        TermbaseScope::Team { team_id } => {
            //
            ListTermbaseInfosExcluded::Team { team_id }
                .step_on(repo, context)
                .await
        }

        TermbaseScope::Comic { comic_id } => {
            //
            ListTermbaseInfosExcluded::Comic { comic_id }
                .step_on(repo, context)
                .await
        }
    }
}

// Apply normalized portable content to one newly-created or existing termbase.
async fn apply_import<C, R>(
    repo: &R,
    context: &mut C,
    token: &UserToken,
    termbase_info: TermbaseInfo,
    termbase_import: TermbaseImport,
    created: bool,
) -> BaseRest<ImportTermbaseVal>
where
    C: Context,
    R: TermbaseRepo<C> + TermRepo<C> + Sync,
{
    let TermbaseImport {
        name,
        description,
        terms,
    } = termbase_import;

    if !created {
        //
        let termbase_info_update = TermbaseRepl {
            id: termbase_info.id.clone(),
            name,
            description,
        };

        UpdateTermbase {
            update: &termbase_info_update,
        }
        .step_on(repo, context)
        .await?;
    }

    let existing_term_infos = ListTermInfos::Termbase {
        termbase_id: &termbase_info.id,
    }
    .step_on(repo, context)
    .await?;

    let term_upsert_plan = TermbaseComplex::build_term_upsert_plan(
        &termbase_info.id,
        &token.user_id,
        termbase_info.term_count,
        &existing_term_infos,
        terms,
    )?;

    let (created_term_count, merged_term_count) = (
        import_count(term_upsert_plan.entries.len())?,
        import_count(term_upsert_plan.updates.len())?,
    );

    UpsertTerms {
        termbase_id: &termbase_info.id,
        entries: &term_upsert_plan.entries,
        updates: &term_upsert_plan.updates,
    }
    .step_on(repo, context)
    .await?;

    if created_term_count > 0 {
        //
        UpdateTermbaseTermCount {
            id: &termbase_info.id,
            delta: i32::try_from(created_term_count).map_err(|_| {
                //
                BaseError::Unrecoverable {
                    message: "created term count exceeds signed delta range"
                        .into(),
                }
            })?,
        }
        .step_on(repo, context)
        .await?;
    }

    accept(ImportTermbaseVal {
        id: termbase_info.id,
        created,
        created_term_count,
        merged_term_count,
    })
}

// Construct a stable expected error when a target scope already owns the imported name.
fn already_exists_err(name: &str) -> BaseError {
    //
    let err_message = trl("error-already-exists");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        termbase_name = %name,
        "expected error: imported termbase already exists",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

// Convert one bounded import count into the public response representation.
const fn import_count(count: usize) -> BaseRest<usize> {
    accept(count)
}
