//! Repository traits for process-local online-user leases.

use poprako_orchestra::drive;

use crate::part::repo::oper::online_user::{ListOnlineUserIds, MarkUserOnline};
use crate::result::BaseError;

/// Online-user repository operations.
///
/// Both operations run independently because leases are process-local and do
/// not participate in database transactions.
#[drive(
    error = BaseError,
    run(for<'a> MarkUserOnline<'a>, for<'a> ListOnlineUserIds<'a>),
)]
pub trait OnlineUserRepo {}
