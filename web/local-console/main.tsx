import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { HomeView } from './HomeView';

function mountLocalHome(): void {
  if (typeof window === 'undefined' || typeof document === 'undefined') return;
  const container = document.getElementById('local-home-root');
  if (!container) return;
  window.__gnomcodeReactViews = {
    ...(window.__gnomcodeReactViews || {}),
    overview: true,
  };
  createRoot(container).render(
    <StrictMode>
      <HomeView />
    </StrictMode>,
  );
}

if (typeof document !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mountLocalHome, { once: true });
  } else {
    mountLocalHome();
  }
}
