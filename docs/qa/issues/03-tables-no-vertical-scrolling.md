# [TUI][Alto] Tabelas não rolam — a linha selecionada some fora da área visível

**Severidade:** 🟠 Alta
**Telas afetadas:** Carteira, Livro de transações, resultados da Busca
**Tipo:** Fluxo / Visual

## Descrição

As tabelas são renderizadas com `ratatui::widgets::Table` aplicando o estilo de
seleção (`Modifier::REVERSED`) manualmente por linha, **sem** `TableState` nem
deslocamento (scroll offset). Quando o número de linhas excede a altura visível
do terminal:

- as linhas extras simplesmente não são desenhadas;
- ao pressionar `↓` além da última linha visível, o índice de seleção continua
  avançando, mas o destaque vai para uma linha **fora da tela** — o usuário deixa
  de ver qual item está selecionado e a tabela **não rola** para acompanhá-lo.

Isso quebra a navegação em qualquer carteira/livro maior que a janela (algo
comum: a carteira de teste tem 21 posições).

## Passos para reproduzir

1. Tenha uma carteira com mais posições do que cabem na altura do terminal
   (ou reduza a altura do terminal).
2. Na Carteira, pressione `↓` repetidamente até passar da última linha visível.

## Resultado esperado

A tabela rola para manter a linha selecionada visível (e idealmente um indicador
de rolagem).

## Resultado obtido

A seleção avança para linhas não renderizadas; o destaque desaparece e a lista
permanece estática.

## Evidência (teste determinístico)

Teste de renderização com `TestBackend` (80×12), 30 linhas, seleção na última:

```
PROBE-SCROLL: top(SYM00) visible=true, selected(SYM29) visible=false
```

A linha selecionada (`SYM29`) não aparece no buffer renderizado, enquanto a
primeira (`SYM00`) continua visível — comprovando que não há rolagem.

## Causa raiz

- `src/tui/screens/portfolio.rs` — `Table::new(...)` sem `TableState`/offset.
- `src/tui/screens/ledger.rs` — mesmo padrão.
- `src/tui/screens/search.rs` — `render_results_table` idem.

A seleção é só um índice (`app.portfolio_selected` / `ledger_selected` /
`search_selected`) usado para estilizar linhas já materializadas.

## Sugestão de correção

Adotar `ratatui::widgets::TableState` com `with_selected(...)` e renderizar via
`frame.render_stateful_widget`, deixando o ratatui cuidar do offset/rolagem; ou
calcular manualmente uma janela (offset) com base na altura disponível e na linha
selecionada.
