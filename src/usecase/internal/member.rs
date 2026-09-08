#[cfg(test)]
mod tests;

use poprako_orchestra::{Context, OperRun as _, OperStep as _};

use poprako_util::i18n::trl;

use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::member_invitation::GetMemberInvitationInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::usecase::internal::util::LoadMode;

/// Loads membership models needed by use-case orchestration.
pub struct MemberLoader;

impl MemberLoader {
    /// Loads a user's membership in the team that owns a comic.
    pub async fn load_info_from_comic<C, R>(
        repo: &R,
        mode: LoadMode<'_, C>,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<MemberInfo>
    where
        C: Context,
        R: MemberRepo<C> + TeamRepo<C> + Sync,
    {
        match mode {
            //
            LoadMode::Run => {
                //
                let team_id =
                    ResolveTeamId::Comic { id: comic_id }.run_on(repo).await?;

                Self::load_info_from_team(
                    repo,
                    LoadMode::Run,
                    user_id,
                    &team_id,
                )
                .await
            }

            LoadMode::Step { context } => {
                //
                let team_id = ResolveTeamId::Comic { id: comic_id }
                    .step_on(repo, context)
                    .await?;

                Self::load_info_from_team(
                    repo,
                    LoadMode::Step { context },
                    user_id,
                    &team_id,
                )
                .await
            }
        }
    }

    /// Loads a user's membership in the team that owns a chapter.
    pub async fn load_info_from_chapter<C, R>(
        repo: &R,
        mode: LoadMode<'_, C>,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseRest<MemberInfo>
    where
        C: Context,
        R: MemberRepo<C> + TeamRepo<C> + Sync,
    {
        match mode {
            //
            LoadMode::Run => {
                //
                let team_id = ResolveTeamId::Chapter { id: chapter_id }
                    .run_on(repo)
                    .await?;

                Self::load_info_from_team(
                    repo,
                    LoadMode::Run,
                    user_id,
                    &team_id,
                )
                .await
            }

            LoadMode::Step { context } => {
                //
                let team_id = ResolveTeamId::Chapter { id: chapter_id }
                    .step_on(repo, context)
                    .await?;

                Self::load_info_from_team(
                    repo,
                    LoadMode::Step { context },
                    user_id,
                    &team_id,
                )
                .await
            }
        }
    }

    /// Loads a user's membership in the team that owns a workset.
    pub async fn load_info_from_workset<C, R>(
        repo: &R,
        mode: LoadMode<'_, C>,
        user_id: &str,
        workset_id: &str,
    ) -> BaseRest<MemberInfo>
    where
        C: Context,
        R: MemberRepo<C> + WorksetRepo<C> + Sync,
    {
        match mode {
            //
            LoadMode::Run => {
                //
                let workset_info =
                    GetWorksetInfo { id: workset_id }.run_on(repo).await?;

                Self::load_info_from_team(
                    repo,
                    LoadMode::Run,
                    user_id,
                    &workset_info.team_id,
                )
                .await
            }

            LoadMode::Step { context } => {
                //
                let workset_info = GetWorksetInfo { id: workset_id }
                    .step_on(repo, context)
                    .await?;

                Self::load_info_from_team(
                    repo,
                    LoadMode::Step { context },
                    user_id,
                    &workset_info.team_id,
                )
                .await
            }
        }
    }

    /// Loads a user's membership in the team that owns an invitation.
    pub async fn load_info_from_member_invitation<C, R>(
        repo: &R,
        mode: LoadMode<'_, C>,
        user_id: &str,
        member_invitation_id: &str,
    ) -> BaseRest<MemberInfo>
    where
        C: Context,
        R: MemberInvitationRepo<C> + MemberRepo<C> + Sync,
    {
        match mode {
            //
            LoadMode::Run => {
                //
                let member_invitation_info = GetMemberInvitationInfo::Id {
                    id: member_invitation_id,
                    incls: &[],
                }
                .run_on(repo)
                .await?;

                Self::load_info_from_team(
                    repo,
                    LoadMode::Run,
                    user_id,
                    &member_invitation_info.team_id,
                )
                .await
            }

            LoadMode::Step { context } => {
                //
                let member_invitation_info = GetMemberInvitationInfo::Id {
                    id: member_invitation_id,
                    incls: &[],
                }
                .step_on(repo, context)
                .await?;

                Self::load_info_from_team(
                    repo,
                    LoadMode::Step { context },
                    user_id,
                    &member_invitation_info.team_id,
                )
                .await
            }
        }
    }

    /// Loads membership for the team resolved from a terminology-base scope.
    pub async fn load_info_from_termbase<C, R>(
        repo: &R,
        mode: LoadMode<'_, C>,
        user_id: &str,
        termbase_info: &TermbaseInfo,
    ) -> BaseRest<MemberInfo>
    where
        C: Context,
        R: MemberRepo<C> + TeamRepo<C> + Sync,
    {
        match (&termbase_info.team_id, &termbase_info.comic_id) {
            //
            (Some(team_id), None) => {
                Self::load_info_from_team(repo, mode, user_id, team_id).await
            }

            (None, Some(comic_id)) => {
                Self::load_info_from_comic(repo, mode, user_id, comic_id).await
            }

            _ => {
                //
                let err_message = trl("error-invalid-termbase-scope");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    termbase_id = %termbase_info.id,
                    team_id = ?termbase_info.team_id,
                    comic_id = ?termbase_info.comic_id,
                    "expected error: invalid termbase ownership scope",
                );

                Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                })
            }
        }
    }

    // Load the final membership operation shared by multi-hop chains.
    async fn load_info_from_team<C, R>(
        repo: &R,
        mode: LoadMode<'_, C>,
        user_id: &str,
        team_id: &str,
    ) -> BaseRest<MemberInfo>
    where
        C: Context,
        R: MemberRepo<C> + Sync,
    {
        let member_info = match mode {
            //
            LoadMode::Run => {
                FindMemberInfo { user_id, team_id }.run_on(repo).await
            }

            LoadMode::Step { context } => {
                //
                FindMemberInfo { user_id, team_id }
                    .step_on(repo, context)
                    .await
            }
        }?;

        Self::require_info(member_info)
    }

    // Require a loaded membership before returning from a loader chain.
    fn require_info(member_info: Option<MemberInfo>) -> BaseRest<MemberInfo> {
        //
        let Some(member_info) = member_info else {
            //
            let err_message = trl("error-team-member-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Perm,
                err_message = %err_message,
                "expected error: team membership required",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: err_message,
            });
        };

        Ok(member_info)
    }
}
