# Remote action activity timer - 2026-11-13

Goal: Authenticated, trusted Remote requests continue_here, worktree_merge, worktree_discard, worktree_open_pr and approvals refresh room activity, alongside existing interrupt and approval requests. Existing authorization and dispatch behavior are unchanged.

Outcome and verification: Encrypted Remote regression first failed on continue_here without a timer refresh. It now checks all seven actions through the real sealed channel before and after room restart/reconnection. Cross-platform review: pure room timing/routing; no OS-specific calls or paths. Physical phone idle-duration acceptance remains unverified.

Decision: reuse the existing Rust contract; no new architecture or dependency.
