use crate::store::{Draft, Store};
use crate::{GroupId, Message, MessageKind, Messages, MessagesError, Speaker};
use core_db::{CommandContext, Storage};

impl Messages {
    pub async fn send(
        &self,
        command_id: String,
        group: &GroupId,
        text: &str,
    ) -> Result<Message, MessagesError> {
        let text = text.trim().to_owned();
        if text.is_empty() || text.chars().count() > 8000 {
            return Err(MessagesError::Invalid(
                "Message must contain 1 to 8000 characters",
            ));
        }
        let group = group.clone();
        let command = CommandContext::user(
            command_id,
            "group.send",
            serde_json::json!({
                "group": group, "text": text,
            }),
        );
        let message = self
            .storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(id) =
                        CommandContext::classify(conn, &command, "Group", group.as_str()).await?
                    {
                        return Store::message(conn, &group, &id).await;
                    }
                    let members = Store::group(conn, &group).await?.members;
                    let to = crate::mentions::addresses(&text, &members, None);
                    let draft = Draft {
                        speaker: Speaker::Person,
                        to,
                        kind: MessageKind::Chat,
                        body: text,
                    };
                    let message = Store::append(conn, &group, draft, None, None).await?;
                    crate::bounds::schedule(conn, &message, message.seq, 1).await?;
                    CommandContext::record_command(
                        conn,
                        &command,
                        "Group",
                        group.as_str(),
                        "GroupMessage",
                        message.id.as_str(),
                        &Storage::now(),
                    )
                    .await?;
                    Ok(message)
                })
            })
            .await?;
        self.wake.notify_one();
        Ok(message)
    }
}
