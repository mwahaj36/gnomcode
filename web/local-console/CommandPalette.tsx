import { useEffect, useMemo, useRef, useState } from 'react';
import { applyThemeMode, navigateTo, openSettingsSearch, revealSection, startOnboarding, workspaceAction } from './shell-bus';
import { sectionMatches, SETTINGS_SECTIONS, signOutDesktop } from './settings-index';

interface Command {
  id: string;
  title: string;
  keywords: string;
  run: () => void;
}

/** Ctrl/Cmd+K palette: views, Settings sections and account actions such as Sign out. */
export function CommandPalette() {
  const dialogRef = useRef<HTMLDialogElement | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);

  const commands = useMemo<Command[]>(
    () => [
      {
        id: 'new-conversation',
        title: 'New conversation',
        keywords: 'launch start session agent chat new',
        run: () => {
          navigateTo('sessions');
          workspaceAction({ type: 'new-session', focus: true });
        },
      },
      {
        id: 'toggle-agent-terminal',
        title: 'Toggle Agent terminal',
        keywords: 'pty stream terminal raw agent output',
        run: () => {
          navigateTo('sessions');
          workspaceAction({ type: 'toggle-agent-terminal' });
        },
      },
      {
        id: 'toggle-overview',
        title: 'Toggle Overview drawer',
        keywords: 'changes files artifacts inspector overview drawer',
        run: () => {
          navigateTo('sessions');
          workspaceAction({ type: 'toggle-overview' });
        },
      },
      {
        id: 'toggle-sidebar',
        title: 'Toggle conversation sidebar',
        keywords: 'conversations sidebar list hide show collapse',
        run: () => {
          navigateTo('sessions');
          workspaceAction({ type: 'toggle-chat-list' });
        },
      },
      {
        id: 'copy-workspace-path',
        title: 'Copy active workspace path',
        keywords: 'copy repo folder directory path clipboard',
        run: () => workspaceAction({ type: 'copy-workspace-path' }),
      },
      { id: 'theme-light', title: 'Switch theme: Light', keywords: 'appearance color light mode day', run: () => applyThemeMode('light') },
      { id: 'theme-dark', title: 'Switch theme: Dark', keywords: 'appearance color dark mode night', run: () => applyThemeMode('dark') },
      { id: 'theme-system', title: 'Switch theme: System', keywords: 'appearance color system auto theme', run: () => applyThemeMode('system') },
      { id: 'sign-out', title: 'Sign out', keywords: 'log out logout disconnect account', run: () => void signOutDesktop() },
      { id: 'home', title: 'Go to Home', keywords: 'overview dashboard', run: () => navigateTo('overview') },
      { id: 'workspace', title: 'Go to Workspace', keywords: 'chats sessions', run: () => navigateTo('sessions') },
      { id: 'usage', title: 'Go to Usage', keywords: 'limits spend activity', run: () => navigateTo('usage') },
      { id: 'setup', title: 'Set up providers', keywords: 'onboarding getting started install', run: () => startOnboarding() },
      ...SETTINGS_SECTIONS.map((section) => ({
        id: section.id,
        title: `Settings: ${section.title}`,
        keywords: section.keywords,
        run: () => {
          openSettingsSearch('');
          window.setTimeout(() => revealSection(section.id), 0);
        },
      })),
    ],
    [],
  );
  const results = commands.filter((command) => sectionMatches(command, query));

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        open();
      }
    };
    const onOpen = () => open();
    window.addEventListener('keydown', onKey);
    window.addEventListener('gnomcode:command-palette', onOpen);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('gnomcode:command-palette', onOpen);
    };
  }, []);

  function open() {
    const dialog = dialogRef.current;
    if (!dialog || dialog.open) return;
    setQuery('');
    setActive(0);
    dialog.showModal();
    inputRef.current?.focus();
  }

  const run = (command: Command | undefined) => {
    if (!command) return;
    dialogRef.current?.close();
    command.run();
  };

  return (
    <dialog id="command-palette" ref={dialogRef} aria-label="Command palette" className="sp-palette">
      <input
        ref={inputRef}
        type="search"
        role="combobox"
        aria-expanded="true"
        aria-controls="command-palette-list"
        aria-activedescendant={results[active] ? `cmd-${results[active].id}` : undefined}
        aria-label="Search commands and settings"
        placeholder="Search commands and settings"
        value={query}
        onChange={(event) => {
          setQuery(event.target.value);
          setActive(0);
        }}
        onKeyDown={(event) => {
          if (event.key === 'ArrowDown') {
            event.preventDefault();
            setActive((i) => Math.min(i + 1, results.length - 1));
          } else if (event.key === 'ArrowUp') {
            event.preventDefault();
            setActive((i) => Math.max(i - 1, 0));
          } else if (event.key === 'Enter') {
            event.preventDefault();
            run(results[active]);
          }
        }}
      />
      <ul id="command-palette-list" role="listbox" aria-label="Commands">
        {results.map((command, index) => (
          <li
            key={command.id}
            id={`cmd-${command.id}`}
            role="option"
            aria-selected={index === active}
            className={index === active ? 'is-active' : undefined}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => run(command)}
          >
            {command.title}
          </li>
        ))}
        {results.length === 0 ? <li role="status">No matches</li> : null}
      </ul>
    </dialog>
  );
}
