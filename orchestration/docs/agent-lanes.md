# Agent Lanes

Use these lanes to avoid collisions while still running in parallel.

## Lane A: Schema and Generation

- Primary repo: `otto-command-service`
- Responsibilities:
  - Command schema edits
  - Generator changes
  - Schema contract validation

## Lane B: Update Surfaces

- Primary repo: `otto-update`
- Responsibilities:
  - Generated surface refresh
  - API and CLI bootstrap validation
  - TypeScript integration fixes

## Lane C: Runtime and Installer

- Primary repo: `otto-update/ottoupdate`
- Responsibilities:
  - Rust runtime behavior
  - Deploy script hardening
  - Packaging pipeline work

## Lane D: Verification and Docs

- Primary repos: all (read-first)
- Responsibilities:
  - Evidence collection
  - Validation reporting
  - Context doc ingestion and task mapping

## Conflict Rules

- One writer per repo per branch at a time.
- Rebase and re-run matrix before merge window.
- No force-push on shared integration branches.
