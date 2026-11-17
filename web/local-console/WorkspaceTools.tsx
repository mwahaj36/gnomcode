'use client';

// Workspace tools carried over from the legacy console so the React Workspace keeps parity:
// attachments (ADR 0041), interrupt (ADR 0035) and the read-only
// Changes, Files and Artifacts panels (ADR 0037, 0050, 0039). Rendering and validation come from
// console-presentation.js; the daemon remains the authority for every limit.
import { useCallback, useEffect, useRef, useState, type MouseEvent } from 'react';
import {
  artifactForLink,
  copyTextToClipboard,
  createInterruptRequestId,
  formatInterruptAckMessage,
  normalizeWorkspaceRelPath,
  renderArtifactPreview,
  renderArtifactsList,
  renderAttachmentPills,
  renderChangesView,
  renderFileViewer,
  validateAttachmentCandidate,
  type ArtifactItem,
  type AttachmentItem,
  type RepoFileListing,
} from '../lib/console-presentation.js';
import { fetchLocalJson, fetchLocalMutation } from './local-api';
import { WorkspaceFileTree } from '../components/WorkspaceFileTree';

/* ---------- Attachments ---------- */

function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = String(reader.result || '');
      const comma = result.indexOf(',');
      resolve(comma >= 0 ? result.slice(comma + 1) : result);
    };
    reader.onerror = () => reject(new Error('Could not read file.'));
    reader.readAsDataURL(file);
  });
}

export function useSessionAttachments(sessionId: string) {
  const [bySession, setBySession] = useState<Record<string, AttachmentItem[]>>({});
  const [status, setStatus] = useState('');
  const items = bySession[sessionId] || [];
  const pending = useRef(new Map<string, string>());

  useEffect(() => setStatus(''), [sessionId]);

  const pick = useCallback(
    async (file: File | null | undefined) => {
      if (!file || !sessionId) return;
      const checked = validateAttachmentCandidate(file);
      if (!checked.ok) {
        setStatus(checked.reason);
        return;
      }
      setStatus('');
      const id = crypto.randomUUID();
      const update = (fn: (list: AttachmentItem[]) => AttachmentItem[]) =>
        setBySession((prev) => ({ ...prev, [sessionId]: fn(prev[sessionId] || []) }));
      pending.current.set(id, sessionId);
      update((list) => [...list, { id, filename: checked.filename, size: checked.size, mime_type: checked.mime_type, status: 'uploading' }]);
      try {
        const data_base64 = await readFileAsBase64(file);
        const res = await fetchLocalMutation<{ workspace_path?: string }>(`/api/v1/sessions/${encodeURIComponent(sessionId)}/attachments`, {
          method: 'POST',
          body: { filename: checked.filename, mime_type: checked.mime_type, size: checked.size, data_base64 },
          timeoutMs: 60000,
        });
        if (!res?.workspace_path) throw new Error('Could not upload attachment.');
        update((list) => list.map((entry) => (entry.id === id ? { ...entry, status: 'saved', workspace_path: res.workspace_path } : entry)));
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Upload failed.';
        update((list) => list.map((entry) => (entry.id === id ? { ...entry, status: 'error', error: message } : entry)));
        setStatus(message);
      } finally {
        pending.current.delete(id);
      }
    },
    [sessionId],
  );

  const remove = useCallback(
    (id: string) => setBySession((prev) => ({ ...prev, [sessionId]: (prev[sessionId] || []).filter((entry) => entry.id !== id) })),
    [sessionId],
  );
  const clearSaved = useCallback((target: string) => {
    setBySession((prev) => ({ ...prev, [target]: (prev[target] || []).filter((entry) => entry.status !== 'saved') }));
    setStatus('');
  }, []);
  return { items, status, pick, remove, clearSaved, blocked: () => [...pending.current.values()].includes(sessionId) || items.some(item => item.status !== 'saved' || !item.workspace_path) };
}

export function AttachmentsBar({ items, status, onRemove }: { items: AttachmentItem[]; status: string; onRemove: (id: string) => void }) {
  const click = (event: MouseEvent<HTMLElement>) => {
    const button = (event.target as HTMLElement).closest('[data-attachment-remove]') as HTMLElement | null;
    if (button?.dataset.attachmentRemove) onRemove(button.dataset.attachmentRemove);
  };
  return (
    <>
      <div
        id="chat-attachments-bar"
        className="sp-attachments-bar"
        aria-label="Staged attachments"
        hidden={!items.length}
        onClick={click}
        dangerouslySetInnerHTML={{ __html: renderAttachmentPills(items) }}
      />
      <p id="chat-attach-status" className="sp-attach-status" role="status">
        {status}
      </p>
    </>
  );
}

/* ---------- Interrupt and read-only panels ---------- */

type PanelKey = 'files' | 'changes' | 'artifacts';

function httpStatusOf(err: unknown): string {
  return err instanceof Error ? err.message : '';
}

export type WorkspaceSidePanel = PanelKey;

export function SessionTools({
  sessionId,
  artifactLink = '',
  repoPath,
  canInterrupt,
  panel: controlledPanel,
  onPanelChange,
  hideToolbar = false,
}: {
  sessionId: string;
  artifactLink?: string;
  repoPath: string;
  canInterrupt: boolean;
  panel?: PanelKey | null;
  onPanelChange?: (next: PanelKey | null) => void;
  hideToolbar?: boolean;
}) {
  const [internalPanel, setInternalPanel] = useState<PanelKey | null>(null);
  const panel = controlledPanel !== undefined ? controlledPanel : internalPanel;
  const setPanel = useCallback(
    (next: PanelKey | null) => {
      if (controlledPanel === undefined) setInternalPanel(next);
      onPanelChange?.(next);
    },
    [controlledPanel, onPanelChange],
  );
  const [changesHtml, setChangesHtml] = useState('');
  const [diffCopied, setDiffCopied] = useState(false);
  const [hasDiffText, setHasDiffText] = useState(false);
  // The last changes payload, so picking a file re-renders without another git call.
  const changesData = useRef<unknown>(null);
  const [fileListing, setFileListing] = useState<RepoFileListing>(null);
  const [fileBodyHtml, setFileBodyHtml] = useState('');
  const [filePath, setFilePath] = useState('');
  const [artifacts, setArtifacts] = useState<ArtifactItem[]>([]);
  const [artifactsBadge, setArtifactsBadge] = useState('');
  const [selectedArtifact, setSelectedArtifact] = useState('');
  const [interruptStatus, setInterruptStatus] = useState('');
  const [interruptBusy, setInterruptBusy] = useState(false);
  const interruptAttempt = useRef<{ sessionId: string; requestId: string } | null>(null);
  const sessionRef = useRef(sessionId);
  sessionRef.current = sessionId;

  useEffect(() => {
    if (controlledPanel === undefined) setInternalPanel(null);
    changesData.current = null;
    setChangesHtml('');
    setHasDiffText(false);
    setDiffCopied(false);
    setFileListing(null);
    setFileBodyHtml('');
    setFilePath('');
    setArtifacts([]);
    setSelectedArtifact('');
    setInterruptStatus('');
    interruptAttempt.current = null;
  }, [sessionId]);

  const loadChanges = useCallback(async () => {
    const target = sessionId;
    setChangesHtml(renderChangesView({ loading: true }));
    try {
      const data = await fetchLocalJson<Record<string, unknown>>(`/api/v1/sessions/${encodeURIComponent(target)}/changes`);
      if (sessionRef.current === target) {
        changesData.current = data;
        setHasDiffText(Boolean(typeof data?.diff === 'string' && data.diff.trim()));
        setChangesHtml(renderChangesView(data as never));
      }
    } catch (err) {
      const text = httpStatusOf(err);
      const message = text.includes('413')
        ? 'Diff exceeded the 1 MiB read limit on this desktop.'
        : text.includes('504')
          ? 'Git diff timed out after 10 seconds.'
          : text.includes('404')
            ? 'This conversation has no workspace on this desktop.'
            : 'This workspace is not a git repository.';
      if (sessionRef.current === target) setChangesHtml(renderChangesView({ error: message }));
    }
  }, [sessionId]);

  const loadFiles = useCallback(
    async (subpath = '') => {
      if (!repoPath) {
        setFileListing({ error: 'Workspace repository path is unavailable for this session.' });
        return;
      }
      setFileListing({ loading: true });
      try {
        const listing = await fetchLocalJson<Record<string, unknown>>(
          `/api/v1/repos/files?repo=${encodeURIComponent(repoPath)}&path=${encodeURIComponent(subpath)}`,
        );
        setFileListing(listing as RepoFileListing);
      } catch (err) {
        setFileListing({ error: httpStatusOf(err).includes('404') ? 'Folder not found inside the workspace.' : 'Could not list this workspace folder.' });
      }
    },
    [repoPath],
  );

  const readFile = useCallback(
    async (raw: string) => {
      const check = normalizeWorkspaceRelPath(raw);
      if (!check.ok) {
        setFileBodyHtml(renderFileViewer({ error: check.error }));
        return;
      }
      if (!repoPath) {
        setFileBodyHtml(renderFileViewer({ error: 'Workspace repository path is unavailable for this session.' }));
        return;
      }
      setFilePath(check.path);
      setFileBodyHtml(renderFileViewer({ loading: true }));
      try {
        const data = await fetchLocalJson<Record<string, unknown>>(
          `/api/v1/repos/file?repo=${encodeURIComponent(repoPath)}&path=${encodeURIComponent(check.path)}`,
        );
        setFileBodyHtml(renderFileViewer(data as never));
      } catch (err) {
        const text = httpStatusOf(err);
        const message = text.includes('413')
          ? 'File exceeded the 512 KiB read cap on desktop.'
          : text.includes('404')
            ? 'File not found inside the workspace.'
            : text.includes('422')
              ? 'Binary or non-UTF-8 files are not shown.'
              : 'File path is outside the workspace or unreadable.';
        setFileBodyHtml(renderFileViewer({ error: message }));
      }
    },
    [repoPath],
  );

  const loadArtifacts = useCallback(async () => {
    const target = sessionId;
    setArtifactsBadge('');
    try {
      const data = await fetchLocalJson<{ artifacts?: ArtifactItem[]; changes_count?: number }>(
        `/api/v1/sessions/${encodeURIComponent(target)}/artifacts`,
      );
      if (sessionRef.current !== target) return;
      const list = Array.isArray(data.artifacts) ? data.artifacts : [];
      setArtifacts(list);
      const linked = artifactLink ? artifactForLink(list, artifactLink) : null;
      const first = artifactLink ? linked : list[0];
      setSelectedArtifact(first ? String(first.id || first.path || 0) : '');
      setArtifactsBadge(
        typeof data.changes_count === 'number' ? `${data.changes_count} workspace change${data.changes_count === 1 ? '' : 's'}` : 'Artifacts',
      );
    } catch {
      if (sessionRef.current === target) {
        setArtifacts([]);
        setArtifactsBadge('Artifacts unavailable for this conversation');
      }
    }
  }, [sessionId, artifactLink]);

  useEffect(() => {
    if (panel === 'changes') void loadChanges();
    if (panel === 'files') void loadFiles('');
    if (panel === 'artifacts') void loadArtifacts();
  }, [panel, loadChanges, loadFiles, loadArtifacts]);

  const toggle = (key: PanelKey) => {
    setPanel(panel === key ? null : key);
  };

  const interrupt = async () => {
    if (interruptBusy) return;
    const target = sessionId;
    if (!interruptAttempt.current || interruptAttempt.current.sessionId !== target) {
      interruptAttempt.current = { sessionId: target, requestId: createInterruptRequestId() };
    }
    setInterruptBusy(true);
    setInterruptStatus('Sending interrupt signal (Esc)...');
    try {
      const ack = await fetchLocalMutation<{ type?: string; message?: string; sent?: boolean }>(
        `/api/v1/sessions/${encodeURIComponent(target)}/interrupt`,
        { method: 'POST', body: { request_id: interruptAttempt.current.requestId } },
      );
      // The daemon answers 200 with { type: 'error', message } when it refuses the interrupt.
      if (ack?.type === 'error') throw new Error(ack.message || 'Could not deliver interrupt. Retry or check Agent terminal.');
      interruptAttempt.current = null;
      if (sessionRef.current === target) setInterruptStatus(formatInterruptAckMessage(ack));
    } catch (err) {
      if (sessionRef.current === target) {
        setInterruptStatus(err instanceof Error ? err.message : 'Interrupt failed. Retry or check Agent terminal.');
      }
    } finally {
      setInterruptBusy(false);
    }
  };

  const delegate = (event: MouseEvent<HTMLElement>) => {
    const el = (event.target as HTMLElement).closest('[data-action]') as HTMLElement | null;
    if (!el) return;
    const arg = el.dataset.arg || '';
    if (el.dataset.action === 'open-repo-dir') void loadFiles(arg);
    if (el.dataset.action === 'open-repo-file') void readFile(arg);
  };

  const selected = artifacts.find((item, idx) => String(item.id || item.path || idx) === selectedArtifact) || null;

  return (
    <>
      {!hideToolbar ? (
        <div className="sp-session-tools" role="toolbar" aria-label="Conversation tools">
          <button type="button" className="btn-control" aria-expanded={panel === 'files'} onClick={() => toggle('files')}>
            Files
          </button>
          <button type="button" className="btn-control" aria-expanded={panel === 'changes'} onClick={() => toggle('changes')}>
            Changes
          </button>
          <button type="button" className="btn-control" aria-expanded={panel === 'artifacts'} onClick={() => toggle('artifacts')}>
            Artifacts
          </button>
          {canInterrupt ? (
            <button
              type="button"
              className="btn-control sp-interrupt-btn"
              aria-label="Interrupt running turn"
              title="Sends Esc to interrupt the active turn without closing the session"
              disabled={interruptBusy}
              onClick={() => void interrupt()}
            >
              Interrupt
            </button>
          ) : null}
        </div>
      ) : null}
      {interruptStatus ? <p role="status">{interruptStatus}</p> : null}

      {panel === 'changes' ? (
        <section className="sp-changes-panel" aria-labelledby="chat-changes-heading">
          <div className="sp-changes-head">
            <h3 id="chat-changes-heading">Workspace changes</h3>
            <div className="sp-changes-actions">
              <button
                type="button"
                className="btn-control"
                disabled={!hasDiffText}
                onClick={() => {
                  const diffText = String((changesData.current as { diff?: string } | null)?.diff || '');
                  if (!diffText) return;
                  void copyTextToClipboard(diffText).then((ok) => {
                    if (!ok) return;
                    setDiffCopied(true);
                    window.setTimeout(() => setDiffCopied(false), 1500);
                  });
                }}
              >
                {diffCopied ? 'Copied' : 'Copy diff'}
              </button>
              <button type="button" className="btn-control" onClick={() => void loadChanges()}>
                Refresh
              </button>
            </div>
          </div>
          <div
            dangerouslySetInnerHTML={{ __html: changesHtml }}
            onClick={(event) => {
              const button = (event.target as HTMLElement).closest<HTMLElement>('[data-change-file]');
              if (!button || !changesData.current) return;
              setChangesHtml(renderChangesView(changesData.current as never, { selected: button.dataset.changeFile || '' }));
            }}
          />
        </section>
      ) : null}

      {panel === 'files' ? (
        <section className="sp-files-panel" aria-labelledby="chat-files-heading" onClick={delegate}>
          <div className="sp-changes-head">
            <h3 id="chat-files-heading">Workspace files</h3>
          </div>
          <form
            className="sp-file-form"
            onSubmit={(event) => {
              event.preventDefault();
              void readFile(filePath);
            }}
          >
            <label className="visually-hidden" htmlFor="chat-file-path-input">
              Relative workspace file path
            </label>
            <input id="chat-file-path-input" type="text" placeholder="src/main.rs" autoComplete="off" value={filePath} onChange={(event) => setFilePath(event.target.value)} />
            <button type="submit" className="btn-control">
              Open
            </button>
          </form>
          <div className="sp-files-layout">
            <WorkspaceFileTree key={sessionId} listing={fileListing} onBrowse={path => void loadFiles(path)} onRead={path => void readFile(path)} selected={filePath} />
            <div className="sp-file-viewer" dangerouslySetInnerHTML={{ __html: fileBodyHtml }} />
          </div>
        </section>
      ) : null}

      {panel === 'artifacts' ? (
        <section className="sp-artifacts-panel" aria-label="Session artifacts alongside changes">
          <header className="sp-artifacts-head">
            <strong>Artifacts</strong>
            <span className="sp-artifacts-changes" role="status">
              {artifactsBadge}
            </span>
            <button type="button" className="btn-control" onClick={() => void loadArtifacts()}>
              Refresh
            </button>
          </header>
          <div className="sp-artifacts-layout">
            <div
              className="sp-artifacts-list"
              onClick={(event) => {
                const el = (event.target as HTMLElement).closest('[data-artifact-id]') as HTMLElement | null;
                if (el?.dataset.artifactId) setSelectedArtifact(el.dataset.artifactId);
              }}
              dangerouslySetInnerHTML={{ __html: renderArtifactsList(artifacts, selectedArtifact) }}
            />
            <div className="sp-artifact-preview" dangerouslySetInnerHTML={{ __html: renderArtifactPreview(selected) }} />
          </div>
        </section>
      ) : null}
    </>
  );
}
