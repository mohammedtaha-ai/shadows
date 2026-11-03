use crate::{Group, GroupId, Message, Messages, MessagesError, store::Store};
use core_db::{CommandContext, Storage};

impl Messages {
    pub async fn create_group(
        &self,
        command_id: String,
        name: &str,
        slugs: &[String],
    ) -> Result<Group, MessagesError> {
        let name = name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 120 {
            return Err(MessagesError::Invalid(
                "Group name must contain 1 to 120 characters",
            ));
        }
        let mut slugs = slugs.to_vec();
        slugs.sort();
        if !(1..=6).contains(&slugs.len()) || slugs.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(MessagesError::Invalid("Choose 1 to 6 distinct agents"));
        }
        if slugs.iter().any(|slug| slug == "mohammed") {
            return Err(MessagesError::Invalid("The Mohammed mention is reserved"));
        }
        let project = self.agents.assistant_project().await?;
        let command = CommandContext::user(
            command_id,
            "group.create",
            serde_json::json!({ "name": name, "members": slugs }),
        );
        // Snapshot names for historical attribution; the thread write rechecks liveness.
        let agents = self.agents.clone();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(id) =
                        CommandContext::classify(conn, &command, "Project", project.as_str())
                            .await?
                    {
                        return Store::group(conn, &GroupId::from_stored(id)).await;
                    }
                    let now = Storage::now();
                    let id = GroupId::generate();
                    Store::insert_group(conn, &id, project.as_str(), &name, &now).await?;
                    for slug in slugs {
                        let agent = agents.agent(&slug).await?;
                        let thread = core_threads::Threads::create_hidden_in(
                            conn, &project, &name, &slug, &now,
                        )
                        .await?;
                        Store::insert_member(conn, &id, &slug, &agent.name, &thread.id).await?;
                    }
                    CommandContext::record_command(
                        conn,
                        &command,
                        "Project",
                        project.as_str(),
                        "MessageGroup",
                        id.as_str(),
                        &now,
                    )
                    .await?;
                    Store::group(conn, &id).await
                })
            })
            .await
    }

    pub async fn groups(&self) -> Result<Vec<Group>, MessagesError> {
        let mut conn = self.storage.reader().begin().await?;
        Store::groups(&mut conn).await
    }

    pub async fn group(&self, id: &GroupId) -> Result<Group, MessagesError> {
        let mut conn = self.storage.reader().begin().await?;
        Store::group(&mut conn, id).await
    }

    pub async fn messages(
        &self,
        group: &GroupId,
        after: Option<i64>,
    ) -> Result<Vec<Message>, MessagesError> {
        let mut conn = self.storage.reader().begin().await?;
        Store::messages(&mut conn, group, after.unwrap_or(0)).await
    }
}
