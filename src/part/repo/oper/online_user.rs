use poprako_orchestra::Oper;

/// Refreshes one user's online lease in a team.
#[derive(Oper)]
#[oper(output = ())]
pub struct MarkUserOnline<'a> {
    //
    /// The team identifier.
    pub team_id: &'a str,

    /// The user identifier.
    pub user_id: &'a str,
}

/// Lists active user identifiers for one team.
#[derive(Oper)]
#[oper(output = Vec<String>)]
pub struct ListOnlineUserIds<'a> {
    /// The team identifier.
    pub team_id: &'a str,
}
