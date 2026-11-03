use crate::{Address, Message, MessagesError, store::Store};
use core_db::Tx;

const MAX_ROUNDS: i64 = 3;
const MAX_DELIVERIES: i64 = 10;
const LIMIT_NOTICE: &str = "The group stopped here: the limit for one message was reached.";

pub(super) async fn schedule(
    conn: &mut Tx,
    message: &Message,
    cause: i64,
    round: i64,
) -> Result<(), MessagesError> {
    let mut count = Store::delivery_count(conn, &message.group_id, cause).await?;
    for address in &message.to {
        let Address::Agent { slug } = address else {
            continue;
        };
        if round > MAX_ROUNDS || count >= MAX_DELIVERIES {
            if !Store::has_notice(conn, &message.group_id, cause, LIMIT_NOTICE).await? {
                Store::append(
                    conn,
                    &message.group_id,
                    Store::notice(LIMIT_NOTICE),
                    Some(cause),
                    None,
                )
                .await?;
            }
            break;
        }
        Store::schedule(conn, message, slug, cause, round).await?;
        count += 1;
    }
    Ok(())
}
