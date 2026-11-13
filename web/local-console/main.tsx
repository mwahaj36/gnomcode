import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ConsoleShell } from './ConsoleShell';

function mountConsole(): void {
  const container = document.getElementById('gnomcode-console-root');
  if (!container) return;
  createRoot(container).render(
    <StrictMode>
      <ConsoleShell />
    </StrictMode>,
  );
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', mountConsole, { once: true });
} else {
  mountConsole();
}
