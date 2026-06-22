# TUI Roadmap + i18n Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evolve the TUI from a two-screen viewer into a navigable portfolio companion with i18n, consistent number formatting, and screen/overlay architecture — without a Settings screen (locale via `config.json` only).

**Architecture:** Screen stack (`Portfolio`, `Detail`, `Search`, `Ledger`) with at most one overlay (modal/toast). Locale and display rules load from `Config` at TUI startup. All user-visible strings come from `src/i18n/` bundles; numeric display uses `core::format` with the active locale.

**Tech Stack:** Rust, ratatui 0.28, crossterm, serde JSON config, embedded locale bundles (no external i18n crate).

**Out of scope (this plan):** Settings screen, `GetConfig`/`UpdateConfig` IPC, charts, Add Transaction modal, daemon validation fixes.

---

## Phase 1 — Foundation (this branch)

### Task 1: Locale in config — DONE (PR #2)

### Task 2: i18n bundles — DONE (PR #2)

### Task 3: Number formatting — DONE (PR #2)

### Task 4: TUI app shell — DONE (PR #2)

### Task 5: Search & Ledger stubs — DONE (PR #2)

### Task 6: Screen decomposition — DONE (PR #3)

Each screen module owns its `render`, `handle_key`, and (for Search) `tick`:

```
src/tui/
  mod.rs          — thin event loop
  models.rs       — shared row types
  state.rs        — UiData
  input.rs        — KeyOutcome
  screens/
    mod.rs        — dispatch render / keys / tick
    portfolio.rs
    detail.rs
    search.rs
    ledger.rs
```

`views.rs` removed.

---

## Phase 2 — Core flows (follow-up PRs)

| Order | Feature | Status |
|-------|---------|--------|
| 1 | Add Transaction modal | DONE (PR #4) |
| 2 | Search with live provider + preview | DONE (PR #5) |
| 3 | Ledger full CRUD + delete confirm | pending |
| 4 | Sort portfolio by score (`o`) | DONE |
| 5 | Braille chart on Detail | pending |
| 6 | Daemon: oversell reject, delete recompute | Independent |

---

## Phase 3 — Deferred

- Settings screen + `GetConfig`/`UpdateConfig` IPC
- Command palette (`Ctrl+P`)
- Import/Export in TUI
- Dashboard as separate screen

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
