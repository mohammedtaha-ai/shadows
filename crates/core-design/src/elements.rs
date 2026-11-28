//! Project table and screen entry points.
use std::collections::HashSet;

use core_db::CommandContext;
use core_grants::Writer;
use sqlx::SqliteConnection;

use crate::{
    Design, DesignElement, DesignError, ElementContent, ElementDelete, ElementInput, Grant,
    ProjectId, ScreenContent, TableContent, store::DesignStore,
};

impl Design {
    /// Writes a new element or appends a revision under the project agent's grant.
    pub async fn put_element_for(
        &self,
        grant: &Grant,
        command_id: String,
        input: ElementInput,
    ) -> Result<DesignElement, DesignError> {
        let (writer, turn) = self
            .proposal_writer(
                grant,
                "design_element_put needs a Planner's conversation",
                "no turn is running for this conversation",
            )
            .await?;
        let write = ElementWrite::Put(input);
        let ctx = write.command(&writer, command_id)?;
        self.store
            .write_element_in(ctx, writer, grant.project_id.clone(), turn, write)
            .await
    }

    /// Appends the last revision of an unreferenced element under a scoped grant.
    pub async fn delete_element_for(
        &self,
        grant: &Grant,
        command_id: String,
        input: ElementDelete,
    ) -> Result<DesignElement, DesignError> {
        let (writer, turn) = self
            .proposal_writer(
                grant,
                "design_element_delete needs a Planner's conversation",
                "no turn is running for this conversation",
            )
            .await?;
        let write = ElementWrite::Delete(input);
        let ctx = write.command(&writer, command_id)?;
        self.store
            .write_element_in(ctx, writer, grant.project_id.clone(), turn, write)
            .await
    }

    /// Every element at its latest revision, including tombstones, by id number.
    pub async fn elements(&self, project: &ProjectId) -> Result<Vec<DesignElement>, DesignError> {
        self.store.elements(project).await
    }

    /// Every revision of an id, oldest first.
    pub async fn element_revisions(
        &self,
        project: &ProjectId,
        id: &str,
    ) -> Result<Vec<DesignElement>, DesignError> {
        self.store.element_revisions(project, id).await
    }

    /// One revision in a caller's snapshot. With no revision, returns the latest.
    pub async fn element_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        id: &str,
        revision: Option<i64>,
    ) -> Result<Option<DesignElement>, DesignError> {
        DesignStore::element_in(conn, project, id, revision).await
    }

    #[cfg(feature = "test-support")]
    pub async fn seed_elements_for_tests(
        &self,
        project: &ProjectId,
    ) -> Result<crate::ElementSeed, DesignError> {
        Ok(self.store.seed_elements_for_tests(project).await?)
    }
}

pub(crate) enum ElementWrite {
    Put(ElementInput),
    Delete(ElementDelete),
}

impl ElementWrite {
    fn command(&self, writer: &Writer, command_id: String) -> Result<CommandContext, DesignError> {
        let (kind, value) = match self {
            Self::Put(input) => ("ElementPut", serde_json::to_value(input)?),
            Self::Delete(input) => ("ElementDelete", serde_json::to_value(input)?),
        };
        let (principal_kind, principal_id) = writer.principal();
        let command_id = if matches!(writer, Writer::External { .. }) {
            Design::external_command_id(Some(command_id))?
        } else {
            command_id
        };
        Ok(CommandContext {
            agent: None,
            principal_kind: principal_kind.into(),
            principal_id,
            command_id,
            command_kind: kind.into(),
            command_schema_ver: 1,
            request_fingerprint: CommandContext::fingerprint(kind, &value),
        })
    }
}

pub(crate) struct ElementRules;

impl ElementRules {
    pub(crate) fn normalise(mut input: ElementInput) -> Result<ElementInput, DesignError> {
        let kind = input.content.kind();
        let (name, purpose) = match &mut input.content {
            ElementContent::Table(table) => (&mut table.name, &mut table.purpose),
            ElementContent::Screen(screen) => (&mut screen.name, &mut screen.purpose),
        };
        for (field, value, max) in [("name", name, 100), ("purpose", purpose, 1000)] {
            trim(value);
            let n = value.chars().count();
            if !(1..=max).contains(&n) {
                return Err(DesignError::InvalidElement(format!(
                    "a {}'s {field} needs 1 to {max} characters, not {n}",
                    kind.as_str()
                )));
            }
        }
        match &mut input.content {
            ElementContent::Table(table) => Self::table(table)?,
            ElementContent::Screen(screen) => Self::screen(screen)?,
        }
        if input.outcomes.is_empty() {
            return Err(DesignError::InvalidElement(
                "an element serves at least one roadmap outcome".into(),
            ));
        }
        let mut outcomes = HashSet::new();
        for id in &input.outcomes {
            if !outcomes.insert(id) {
                return Err(DesignError::InvalidElement(format!(
                    "outcome {id} is named twice"
                )));
            }
        }
        if let Some(id) = &mut input.id {
            trim(id);
        }
        match (&input.id, input.expected_revision) {
            (None, Some(_)) => Err(DesignError::InvalidElement(
                "a new element has no expected_revision".into(),
            )),
            (Some(id), None) => Err(DesignError::ElementStale(format!(
                "{id} needs the expected_revision of its latest revision"
            ))),
            _ => Ok(input),
        }
    }

    fn table(table: &mut TableContent) -> Result<(), DesignError> {
        let n = table.columns.len();
        if !(1..=200).contains(&n) {
            return Err(DesignError::InvalidElement(format!(
                "a table needs 1 to 200 columns, not {n}"
            )));
        }
        let mut names = HashSet::new();
        for (index, column) in table.columns.iter_mut().enumerate() {
            trim(&mut column.name);
            trim(&mut column.type_name);
            trim(&mut column.meaning);
            if !(1..=100).contains(&column.name.chars().count()) {
                return Err(DesignError::InvalidElement(format!(
                    "column {} needs a name of 1 to 100 characters",
                    index + 1
                )));
            }
            let name = &column.name;
            if !(1..=100).contains(&column.type_name.chars().count()) {
                return Err(DesignError::InvalidElement(format!(
                    "the column `{name}` needs a type of 1 to 100 characters"
                )));
            }
            if column.meaning.chars().count() > 500 {
                return Err(DesignError::InvalidElement(format!(
                    "the column `{name}`'s meaning is over 500 characters"
                )));
            }
            if !names.insert(name.to_lowercase()) {
                return Err(DesignError::InvalidElement(format!(
                    "the column `{name}` is named twice"
                )));
            }
            if let Some(target) = &mut column.references {
                trim(&mut target.table);
                trim(&mut target.column);
            }
        }
        Ok(())
    }

    fn screen(screen: &mut ScreenContent) -> Result<(), DesignError> {
        if let Some(route) = &mut screen.route {
            trim(route);
            if !route.starts_with('/') || route.chars().count() > 200 {
                return Err(DesignError::InvalidElement(format!(
                    "a route starts with '/' and has up to 200 characters: `{route}`"
                )));
            }
        }
        let n = screen.leads_to.len();
        if n > 50 {
            return Err(DesignError::InvalidElement(format!(
                "a screen leads to at most 50 screens, not {n}"
            )));
        }
        let mut links = HashSet::new();
        for link in &mut screen.leads_to {
            trim(&mut link.screen);
            trim(&mut link.when);
            let target = &link.screen;
            if !(1..=300).contains(&link.when.chars().count()) {
                return Err(DesignError::InvalidElement(format!(
                    "the link to {target} needs a when of 1 to 300 characters"
                )));
            }
            if !links.insert((&link.screen, &link.when)) {
                return Err(DesignError::InvalidElement(format!(
                    "{target} is linked twice with the same when"
                )));
            }
        }
        Ok(())
    }
}

fn trim(value: &mut String) {
    *value = value.trim().to_owned();
}

#[cfg(test)]
mod tests {
    use super::ElementRules;
    use crate::{
        ElementContent, ElementInput, OutcomeId, PartId, ScreenContent, TableColumn, TableContent,
    };

    fn column(name: &str) -> TableColumn {
        TableColumn {
            name: name.into(),
            type_name: " TEXT ".into(),
            meaning: " Primary identity ".into(),
            primary_key: true,
            references: None,
        }
    }

    fn input(content: ElementContent) -> ElementInput {
        ElementInput {
            id: None,
            expected_revision: None,
            part_id: PartId::generate(),
            outcomes: vec![OutcomeId::generate()],
            content,
        }
    }

    #[test]
    fn names_and_texts_are_trimmed_and_bounded() {
        let mut table = TableContent {
            name: "x".repeat(101),
            purpose: " Local users ".into(),
            columns: vec![column(" id ")],
        };
        let err = ElementRules::normalise(input(ElementContent::Table(table.clone()))).unwrap_err();
        assert_eq!(
            err.to_string(),
            "a table's name needs 1 to 100 characters, not 101"
        );
        table.name = "  users ".into();
        let element = ElementRules::normalise(input(ElementContent::Table(table))).unwrap();
        let ElementContent::Table(table) = element.content else {
            panic!("expected a table");
        };
        assert_eq!(table.name, "users");
        assert_eq!(table.purpose, "Local users");
        assert_eq!(table.columns[0].name, "id");
        assert_eq!(table.columns[0].type_name, "TEXT");
        assert_eq!(table.columns[0].meaning, "Primary identity");
    }

    #[test]
    fn a_column_name_is_unique_ignoring_case() {
        let args = input(ElementContent::Table(TableContent {
            name: "users".into(),
            purpose: "Local users".into(),
            columns: vec![column("id"), column("ID")],
        }));
        let err = ElementRules::normalise(args).unwrap_err();
        assert_eq!(err.to_string(), "the column `ID` is named twice");
    }

    #[test]
    fn a_route_starts_with_a_slash() {
        let mut screen = ScreenContent {
            name: "Login".into(),
            purpose: "Sign in".into(),
            route: Some("login".into()),
            leads_to: vec![],
        };
        let err =
            ElementRules::normalise(input(ElementContent::Screen(screen.clone()))).unwrap_err();
        assert_eq!(
            err.to_string(),
            "a route starts with '/' and has up to 200 characters: `login`"
        );
        screen.route = Some(" /login ".into());
        let args = ElementRules::normalise(input(ElementContent::Screen(screen))).unwrap();
        let ElementContent::Screen(screen) = args.content else {
            panic!("expected a screen");
        };
        assert_eq!(screen.route.as_deref(), Some("/login"));
    }
}
