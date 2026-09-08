//! Terminology-base use cases.

#[cfg(test)]
// Unit tests for terminology base definitions and search access.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::comic::ComicComplex;
use crate::complex::termbase::{TermbaseComplex, TermbasePermComplex};
use crate::data::instr::termbase::{
    CreateTermbaseInstr, ListComicTermbaseInfosInstr,
    ListTeamTermbaseInfosInstr, UpdateTermbaseInfoInstr,
};
use crate::data::val::termbase::CreateTermbaseVal;
use crate::data::view::termbase::TermbaseInfoView;
use crate::model::read::spec::termbase::TermbaseListSpec;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::GetComicInfoExcluded;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::LockTeam;
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, GetTermbaseInfo, GetTermbaseInfoExcluded,
    ListTermbaseInfos, UpdateTermbase,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

/// Creates a terminology base scoped to a team or comic.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateTermbaseInstr,
) -> BaseRest<CreateTermbaseVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TeamRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + TermbaseRepo<C>
        + Send
        + Sync,
{
    let termbase_entry = TermbaseComplex::build_entry(
        instr.team_id,
        instr.comic_id,
        instr.name,
        instr.description,
        token.user_id.clone(),
    )?;

    let termbase_id = nucl
        .coord(async move |context| {
            //
            let team_id =
                match (&termbase_entry.team_id, &termbase_entry.comic_id) {
                    //
                    (Some(team_id), None) => {
                        //
                        LockTeam { id: team_id }.step_on(repo, context).await?;

                        team_id.clone()
                    }

                    (None, Some(comic_id)) => {
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

                        workset_info.team_id
                    }

                    _ => {
                        //
                        return Err(BaseError::Unrecoverable {
                            message: "validated termbase scope is invalid"
                                .into(),
                        });
                    }
                };

            let member_info = FindMemberInfo {
                user_id: &token.user_id,
                team_id: &team_id,
            }
            .step_on(repo, context)
            .await?;

            let Some(member_info) = member_info else {
                //
                let err_message =
                    trl("error-team-translator-or-proofreader-required");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    team_id = %team_id,
                    user_id = %token.user_id,
                    "expected error: termbase creator membership missing",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                });
            };

            TermbasePermComplex::ensure_user_can_write_team(&member_info)?;

            let termbase_info = CreateTermbase {
                entry: &termbase_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(termbase_info.id)
        })
        .await?;

    accept(CreateTermbaseVal { id: termbase_id })
}

/// Fetches a terminology base by ID.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn get_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    id: String,
) -> BaseRest<TermbaseInfoView>
where
    C: Context,
    R: TermbaseRepo<C> + TeamRepo<C> + MemberRepo<C> + Sync,
{
    let termbase_info = GetTermbaseInfo { id: &id }.run_on(repo).await?;

    let member_info = MemberLoader::load_info_from_termbase(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &termbase_info,
    )
    .await?;

    TermbasePermComplex::ensure_user_can_read(&member_info, &termbase_info)?;

    accept(termbase_info.into())
}

/// Lists terminology bases directly owned by a team.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn list_team_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListTeamTermbaseInfosInstr,
) -> BaseRest<Vec<TermbaseInfoView>>
where
    C: Context,
    R: TermbaseRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &instr.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-member-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            team_id = %instr.team_id,
            user_id = %token.user_id,
            "expected error: termbase list membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    TermbasePermComplex::ensure_user_can_read_team(&member_info)?;

    let termbase_info_list_spec = TermbaseListSpec::Team {
        team_id: instr.team_id,
        fuzzy_name: TermbaseComplex::normalize_fuzzy_name(instr.fuzzy_name),
        offset: instr.offset,
        limit: instr.limit,
    };

    let termbase_infos = ListTermbaseInfos {
        spec: &termbase_info_list_spec,
    }
    .run_on(repo)
    .await?;

    accept(termbase_infos.into_iter().map(Into::into).collect())
}

/// Lists team and comic terminology bases visible from a comic.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn list_comic_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListComicTermbaseInfosInstr,
) -> BaseRest<Vec<TermbaseInfoView>>
where
    C: Context,
    R: TermbaseRepo<C> + TeamRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &instr.comic_id,
    )
    .await?;

    TermbasePermComplex::ensure_user_can_read_comic(&member_info)?;

    let termbase_info_list_spec = TermbaseListSpec::Comic {
        comic_id: instr.comic_id,
        fuzzy_name: TermbaseComplex::normalize_fuzzy_name(instr.fuzzy_name),
        offset: instr.offset,
        limit: instr.limit,
    };

    let termbase_infos = ListTermbaseInfos {
        spec: &termbase_info_list_spec,
    }
    .run_on(repo)
    .await?;

    accept(termbase_infos.into_iter().map(Into::into).collect())
}

/// Replaces a terminology base's name and description.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateTermbaseInfoInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TermbaseRepo<C> + TeamRepo<C> + MemberRepo<C> + Send + Sync,
{
    let termbase_info_update =
        TermbaseComplex::build_update(instr.id, instr.name, instr.description)?;

    nucl.coord(async move |context| {
        //
        let termbase_info = GetTermbaseInfoExcluded {
            id: &termbase_info_update.id,
        }
        .step_on(repo, context)
        .await?;

        let member_info = MemberLoader::load_info_from_termbase(
            repo,
            LoadMode::Step { context },
            &token.user_id,
            &termbase_info,
        )
        .await?;

        TermbasePermComplex::ensure_user_can_write(
            &member_info,
            &termbase_info,
        )?;

        UpdateTermbase {
            update: &termbase_info_update,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes a terminology base and all child terms.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
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
    nucl.coord(async move |context| {
        //
        let termbase_info = GetTermbaseInfoExcluded { id: &id }
            .step_on(repo, context)
            .await?;

        let member_info = MemberLoader::load_info_from_termbase(
            repo,
            LoadMode::Step { context },
            &token.user_id,
            &termbase_info,
        )
        .await?;

        TermbasePermComplex::ensure_user_can_write(
            &member_info,
            &termbase_info,
        )?;

        delete_cascade(repo, context, &termbase_info.id).await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes one terminology base and all child terms in a transaction.
pub async fn delete_cascade<C, R>(
    repo: &R,
    context: &mut C,
    id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: TermbaseRepo<C> + TermRepo<C> + Sync,
{
    let termbase_info = GetTermbaseInfoExcluded { id }
        .step_on(repo, context)
        .await?;

    DeleteTerms {
        termbase_id: &termbase_info.id,
    }
    .step_on(repo, context)
    .await?;

    DeleteTermbase {
        id: &termbase_info.id,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}
