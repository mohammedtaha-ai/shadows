CREATE TABLE aylul_agent_mcp_tool_rule (
    agent_slug TEXT NOT NULL REFERENCES aylul_agent(slug),
    mcp_id TEXT NOT NULL REFERENCES aylul_mcp_server(id) ON DELETE CASCADE,
    tool TEXT NOT NULL,
    rule TEXT NOT NULL CHECK (rule IN ('allow', 'deny')),
    PRIMARY KEY (agent_slug, mcp_id, tool)
);
