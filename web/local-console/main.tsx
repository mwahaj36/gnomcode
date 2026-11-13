import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { HomeView } from './HomeView';
import { UsageView } from './UsageView';
import { WorkspaceView } from './WorkspaceView';

function mountLocalConsoleViews(): void {
  if (typeof window === 'undefined' || typeof document === 'undefined') return;

  const homeContainer = document.getElementById('local-home-root');
  if (homeContainer) {
    window.__gnomcodeReactViews = {
      ...(window.__gnomcodeReactViews || {}),
      overview: true,
    };
    createRoot(homeContainer).render(
      <StrictMode>
        <HomeView />
      </StrictMode>,
    );
  }

  const workspaceContainer = document.getElementById('local-workspace-root');
  if (workspaceContainer) {
    window.__gnomcodeReactViews = {
      ...(window.__gnomcodeReactViews || {}),
      sessions: true,
    };
    const legacyWorkspace = document.getElementById('legacy-ide-layout');
    if (legacyWorkspace) {
      legacyWorkspace.hidden = true;
    }
    createRoot(workspaceContainer).render(
      <StrictMode>
        <WorkspaceView />
      </StrictMode>,
    );
  }

  const usageContainer = document.getElementById('local-usage-root');
  if (usageContainer) {
    window.__gnomcodeReactViews = {
      ...(window.__gnomcodeReactViews || {}),
      usage: true,
    };
    document.querySelectorAll<HTMLElement>('[data-legacy-usage="true"]').forEach((node) => {
      node.hidden = true;
    });
    createRoot(usageContainer).render(
      <StrictMode>
        <UsageView />
      </StrictMode>,
    );
  }
}

if (typeof document !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mountLocalConsoleViews, { once: true });
  } else {
    mountLocalConsoleViews();
  }
}
