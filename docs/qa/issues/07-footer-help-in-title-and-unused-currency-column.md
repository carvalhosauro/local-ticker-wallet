# [TUI][Baixo] Texto de ajuda no título da borda + coluna "Moeda" nunca exibida na Busca

**Severidade:** 🔵 Baixa
**Telas afetadas:** Todas (título) / Busca (coluna)
**Tipo:** Visual / UX

## Item A — Ajuda renderizada no título da borda (topo), não num rodapé

Em todas as telas, o texto de ajuda (com hints de teclas, ex.: "↑↓ navegar ·
Enter detalhe · … · q sair") é colocado como **título da borda superior** do
bloco, e não num rodapé/barra de status na parte de baixo. As constantes têm
nome `*_footer` (sugerindo rodapé), mas aparecem no topo.

- `src/tui/screens/portfolio.rs`: `title = format!("{} — {}", app_title, portfolio_footer)`
- `src/tui/screens/detail.rs`, `ledger.rs`: mesmo padrão.
- `src/tui/screens/search.rs`: `render_results_table` usa `.title(bundle.search_footer)` — o texto de ajuda vira o título da tabela de resultados.

Sugestão: renderizar os hints numa linha de rodapé/barra de status fixa na base,
condizente com o nome `*_footer` e com a convenção de TUIs.

## Item B — Coluna "Moeda" definida porém nunca exibida nos resultados da Busca

Os bundles definem `search_col_currency` ("Moeda"/"Currency") e o modelo
`SearchResultRow` carrega `currency`, mas a tabela de resultados
(`render_results_table` em `src/tui/screens/search.rs`) só mostra **Ativo / Nome
/ Tipo**. A moeda aparece apenas no painel "Prévia". Resultado: string e dado
carregados sem uso na lista, e o usuário não vê a moeda ao comparar resultados
(relevante quando a busca mistura ativos BRL e USD).

Sugestão: ou exibir a coluna "Moeda" na tabela de resultados, ou remover a
string/constante não utilizada para evitar confusão.

## Evidência

Screenshots `screenshot_portfolio_overview.webp` e
`screenshot_search_vale_typed.webp`: hints no título da borda superior; tabela de
resultados sem coluna de moeda (apenas Ativo/Nome/Tipo).
