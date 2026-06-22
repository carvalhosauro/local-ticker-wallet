# TUI Roadmap + i18n Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evolve the TUI from a two-screen viewer into a navigable portfolio companion with i18n, consistent number formatting, and screen/overlay architecture — without a Settings screen (locale via `config.json` only).

**Architecture:** Screen stack (`Portfolio`, `Detail`, `Search`, `Ledger`) with at most one overlay (modal/toast). Locale and display rules load from `Config` at TUI startup. All user-visible strings come from `src/i18n/` bundles; numeric display uses `core::format` with the active locale.

**Tech Stack:** Rust, ratatui 0.28, crossterm, serde JSON config, embedded locale bundles (no external i18n crate).

**Out of scope (this plan):** Settings screen, `GetConfig`/`UpdateConfig` IPC, charts, Add Transaction modal, daemon validation fixes.

---

## Phase 1 — Foundation (this branch)

### Task 1: Locale in config

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs` (unit tests)

- [ ] Add `Locale` enum (`pt-BR`, `en`) with serde `rename_all = "kebab-case"`
- [ ] Add `locale: Locale` to `Config` (default `pt-BR`)
- [ ] Test partial JSON and roundtrip

### Task 2: i18n bundles

**Files:**
- Create: `src/i18n/mod.rs`, `src/i18n/pt_br.rs`, `src/i18n/en.rs`
- Modify: `src/lib.rs`

- [ ] `Bundle` struct with all TUI strings (nav, columns, detail labels, score names, hints, errors)
- [ ] `bundle(locale) -> &'static Bundle`
- [ ] Human-readable score sub-score labels (not code names)

### Task 3: Number formatting

**Files:**
- Create: `src/core/format.rs`
- Modify: `src/core/mod.rs`

- [ ] `FormatLocale` mirroring config locale
- [ ] `format_price`, `format_money`, `format_pct`, `format_quantity`, `format_score_sub`
- [ ] Unit tests for pt-BR and en separators

### Task 4: TUI app shell

**Files:**
- Create: `src/tui/app.rs`, `src/tui/screens/mod.rs`, `src/tui/widgets/mod.rs`, `src/tui/widgets/status_bar.rs`, `src/tui/widgets/toast.rs`
- Modify: `src/tui/mod.rs`, `src/tui/views.rs`, `src/tui/client.rs`

- [ ] `Screen` enum: Portfolio, Detail, Search, Ledger
- [ ] `App` holds locale, bundle, screen, toast, selection state
- [ ] Status bar with tab hints `[1] Portfolio [2] Search [3] Ledger`
- [ ] Toast for IPC errors (replace silent `unwrap_or_default`)
- [ ] Load `Config::load()` at TUI start for locale

### Task 5: Search & Ledger stubs

**Files:**
- Create: `src/tui/screens/search.rs`, `src/tui/screens/ledger.rs`, `src/tui/screens/portfolio.rs`, `src/tui/screens/detail.rs`

- [ ] Search: query input placeholder, empty results area, footer hints
- [ ] Ledger: placeholder “coming soon” or basic `ListTransactions` table
- [ ] Navigation: `1`/`2`/`3`, `/` jumps to Search, `Esc` back

---

## Phase 2 — Core flows (follow-up PRs)

| Order | Feature | Depends on |
|-------|---------|------------|
| 1 | Add Transaction modal | Phase 1 shell |
| 2 | Search with live provider + preview | Phase 1 Search screen |
| 3 | Ledger full CRUD + delete confirm | Phase 1 Ledger screen |
| 4 | Sort portfolio by score (`o`) | Phase 1 |
| 5 | Braille chart on Detail | Detail screen |
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
