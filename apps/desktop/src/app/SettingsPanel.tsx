import React from 'react';

type Props = {account: React.ReactNode; studio?: React.ReactNode; footer?: React.ReactNode};

// Settings groups. Each slot is an independent owner: one failing never disables another.
export function SettingsPanel({account, studio, footer}: Props) {
  return <>
    <h1>Settings</h1>
    {account}
    <section aria-labelledby="settings-appearance"><h2 id="settings-appearance">Appearance</h2>
      <p>The desktop character is shown by its own window. Show, hide or pause it with the AYLUL controls in the top bar.</p></section>
    <section aria-labelledby="settings-integrations"><h2 id="settings-integrations">Integrations</h2>
      <p>Local agent sessions and requests are under Activity.</p>
      <button type="button" disabled aria-describedby="settings-tools-reason">Skills, tools and MCP</button>
      <small id="settings-tools-reason"> Not available yet.</small></section>
    {studio}
    {footer}
  </>;
}
