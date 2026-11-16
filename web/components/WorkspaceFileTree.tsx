'use client';

import { useEffect, useState } from 'react';
import type { RepoFileListing } from '../lib/console-presentation';

/** Shared lazy folder outline; local and encrypted Remote keep their own transports. */
export function WorkspaceFileTree({ listing, onBrowse, onRead, selected }: {
  listing: RepoFileListing;
  onBrowse: (path: string) => void;
  onRead: (path: string) => void;
  selected: string;
}) {
  const [folders, setFolders] = useState<Record<string, NonNullable<RepoFileListing>>>({});
  const [expanded, setExpanded] = useState<string[]>(['']);
  useEffect(() => {
    if (listing && !listing.loading && !listing.error) {
      setFolders(current => ({ ...current, [listing.path || '']: listing }));
    }
  }, [listing]);

  const rows = (path: string, depth = 0): React.ReactNode => (folders[path]?.entries || []).map(entry => {
    const folder = entry.kind === 'dir';
    const open = expanded.includes(entry.path);
    return <li key={entry.path}>
      <button type="button" className="sp-file-tree-row" style={{ paddingInlineStart: 12 + depth * 16 }}
        aria-expanded={folder ? open : undefined} aria-current={!folder && selected === entry.path ? 'true' : undefined}
        onClick={() => {
          if (!folder) { onRead(entry.path); return; }
          setExpanded(current => open ? current.filter(value => value !== entry.path) : [...current, entry.path]);
          if (!open) onBrowse(entry.path);
        }}>
        <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true">
          <path d={folder ? (open ? 'm5 8 5 5 5-5' : 'm8 5 5 5-5 5') : 'M5 2h7l3 3v13H5z M11 2v4h4'} />
        </svg><span>{entry.name}</span>
      </button>
      {folder && open && <ul>{rows(entry.path, depth + 1)}</ul>}
    </li>;
  });
  return <nav className="sp-file-tree" aria-label="Workspace files">
    <ul>{rows('')}</ul>
    {listing?.loading && <p role="status" className="sp-field-note">Loading workspace files...</p>}
    {listing?.error && <p role="alert" className="sp-field-note sp-tone-red">{listing.error}</p>}
    {listing?.note && <p className="sp-field-note">{listing.note}</p>}
    {listing?.truncated && <p className="sp-field-note">Folder listing truncated. Open a file by its relative path.</p>}
    {folders['']?.entries?.length === 0 && <p className="sp-provider-empty">No files listed here. Open a file by its relative path.</p>}
  </nav>;
}
