export interface WorktreeDiffData {
  loading?: boolean;
  error?: string;
  repo?: string;
  branch?: string;
  status?: string;
  diff?: string;
  lossy?: boolean;
}

export function formatWorktreeBadge(session?: {
  worktree?: boolean;
  worktree_branch?: string | null;
  worktree_path?: string | null;
} | null): string;

export function renderWorktreeDiffMarkup(data?: WorktreeDiffData | null): string;
