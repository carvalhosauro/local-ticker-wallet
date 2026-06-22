# TUI Roadmap + i18n Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evolve the TUI from a two-screen viewer into a navigable portfolio companion with i18n, consistent number formatting, and screen/overlay architecture — without a Settings screen (locale via `config.json` only).

**Architecture:** Screen stack (`Portfolio`, `Detail`, `Search`, `Ledger`) with at most one overlay (modal/toast). Locale and display rules load from `Config` at TUI startup. All user-visible strings come from `src/i18n/` bundles; numeric display uses `core::format` with the active locale.

**Tech Stack:** Rust, ratatui 0.28, crossterm, serde JSON config, embedded locale bundles (no external i18n crate).

**Out of scope (this plan):** Settings screen, `GetConfig`/`UpdateConfig` IPC.

---

## Phase 1 — Foundation — DONE

All tasks shipped in PRs #2–#3 (i18n, formatting, screen stack, decomposition).

---

## Phase 2 — Core flows

| Order | Feature | Status |
|-------|---------|--------|
| 1 | Add Transaction modal | DONE (PR #4) |
| 2 | Search with live provider + preview | DONE (PR #5) |
| 3 | Ledger delete + confirm (`d`) | DONE (PR #6) — create via modal (`a`); **edit (U) not implemented** |
| 4 | Sort portfolio by score (`o`) | DONE |
| 5 | Braille chart on Detail | **PR open** — branch `cursor/detail-braille-chart-e195` not merged to `main` yet |
| 6a | Daemon: delete → recompute snapshot | DONE (PR #6) |
| 6b | Daemon: reject oversell on `AddTransaction` | **Pending** — spec gap, not a crash bug (see below) |

### Oversell reject — bug or feature gap?

The design spec requires rejecting sells above held quantity. `core::pnl` already returns `PnlError::Oversell`, but `AddTransaction` **persists the trade first** and only logs a warning if `recompute_asset` fails. The API still returns `{"id": …}`.

This is an **implementation gap vs. the spec** (data integrity), not a runtime crash. A user can end up with an invalid ledger row and a stale/missing snapshot. Fixing it means validating before insert (or rolling back in a transaction) and returning `BadRequest` to CLI/TUI.

---

## Phase 3 — Deferred

- Settings screen + `GetConfig`/`UpdateConfig` IPC
- Command palette (`Ctrl+P`)
- Import/Export in TUI
- Dashboard as separate screen
- Ledger edit transaction (completes CRUD **U**)
- Fix `tests/e2e.rs` hang (daemon stdout pipe)

---

## i18n policy

- **Source of truth:** Rust modules `pt_br.rs` / `en.rs` (compile-time, no runtime file IO).
- **Selection:** `config.json` → `"locale": "pt-BR"` | `"en"`.
- **Fallback:** Unknown locale string deserializes to `pt-BR` via serde default.
- **CLI:** Unaffected in Phase 1; English/Portuguese only for TUI.
- **Future:** Add `es` etc. by adding a new module + enum variant.

## Format policy

| Type | pt-BR | en |
|------|-------|-----|
| Price | `28,50` | `28.50` |
| Money | `R$ 8.106,00` | `$8,106.00` |
| Percent | `+1,25%` | `+1.25%` |
| Quantity | int or up to 4 dp | same |
| Score | integer | integer |
