import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { HomeView } from './HomeView';
import { SettingsView } from './SettingsView';
import { UsageView } from './UsageView';
import { WorkspaceView } from './WorkspaceView';

const VIEWS = [
  ['local-home-root', HomeView],
  ['local-workspace-root', WorkspaceView],
  ['local-usage-root', UsageView],
  ['local-settings-root', SettingsView],
] as const;

function mountLocalConsoleViews(): void {
  for (const [id, View] of VIEWS) {
    const container = document.getElementById(id);
    if (!container) continue;
    createRoot(container).render(
      <StrictMode>
        <View />
      </StrictMode>,
    );
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', mountLocalConsoleViews, { once: true });
} else {
  mountLocalConsoleViews();
}
