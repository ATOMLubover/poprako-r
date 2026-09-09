use diesel::{QueryDsl as _, TextExpressionMethods as _};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbCore;

use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{BaseRest, accept};
use crate::shared::result::diesel as diesel_error;

pub async fn reset(shared: &RdbCore, prefix: &str) {
    //
    cleanup(shared, prefix).await.unwrap();

    assert_no_leftovers(shared, prefix).await.unwrap();
}

pub async fn cleanup(shared: &RdbCore, prefix: &str) -> BaseRest<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    diesel::delete(
        t_local_message::table.filter(t_local_message::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    accept(())
}

pub async fn assert_no_leftovers(
    shared: &RdbCore,
    prefix: &str,
) -> BaseRest<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    let local_message_count = t_local_message::table
        .filter(t_local_message::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    assert_eq!(local_message_count, 0);

    accept(())
}
