# QA — Relatório de testes de interface da TUI (2026-06-22)

Bateria de testes de ponta a ponta da TUI (`ltw tui`), cobrindo todas as telas
(Carteira, Detalhe, Busca, Livro) e overlays (Nova transação, Confirmar
exclusão). O objetivo foi caçar bugs de **valores**, **fluxos** e **visuais**.

Os testes combinaram:

- **Teste manual** na TUI real (terminal), com carteira isolada e ~21 posições
  semeadas (`XDG_*` apontando para um diretório descartável).
- **Análise de código** dos manipuladores de teclas e renderizadores.
- **Testes de renderização determinísticos** (ratatui `TestBackend`) para
  comprovar bugs de layout/formatação.

> Observação: este agente não tem permissão para abrir issues no GitHub
> diretamente. Cada bug abaixo está escrito como uma issue pronta para ser
> copiada/colada no rastreador de issues do projeto.

## Índice de issues encontradas

| # | Severidade | Título |
|---|------------|--------|
| [01](issues/01-search-field-swallows-navigation-keys.md) | 🔴 Crítica | Tela de Busca engole teclas de navegação (`1` `2` `3` `/` `q`) — impossível buscar tickers da B3 |
| [02](issues/02-q-key-quits-app-inside-modals.md) | 🔴 Crítica | `q` dentro dos modais (Nova transação / Confirmar exclusão) fecha o app inteiro |
| [03](issues/03-tables-no-vertical-scrolling.md) | 🟠 Alta | Tabelas (Carteira/Livro/Busca) não rolam — a linha selecionada some fora da área visível |
| [04](issues/04-search-preview-stuck-loading-on-error.md) | 🟡 Média | Painel de prévia da Busca fica preso em "Carregando cotação…" após falha |
| [05](issues/05-sort-by-score-toggle-not-reversible.md) | 🟡 Média | `o` (ordenar por score) não desfaz a ordenação e não reposiciona a seleção |
| [06](issues/06-negative-zero-number-formatting.md) | 🔵 Baixa | Formatação exibe `-0,00%` / `-R$ 0,00` (sinal de menos enganoso) |
| [07](issues/07-footer-help-in-title-and-unused-currency-column.md) | 🔵 Baixa | Texto de ajuda no título da borda + coluna "Moeda" definida porém nunca exibida na Busca |

## Como reproduzir o ambiente de teste

```bash
export XDG_DATA_HOME=/tmp/ltw-test/data
export XDG_CONFIG_HOME=/tmp/ltw-test/config
export XDG_RUNTIME_DIR=/tmp/ltw-test/run
# semear cotações/candles + lançar transações, então:
ltw tui
```

As issues 01, 02 e 03 são as mais impactantes: as duas primeiras tornam a Busca
e os modais praticamente inutilizáveis para o caso de uso real (tickers da B3
quase sempre terminam em dígito, e `q` é uma letra comum), e a terceira quebra
a navegação assim que a carteira/livro passa do tamanho da tela.
