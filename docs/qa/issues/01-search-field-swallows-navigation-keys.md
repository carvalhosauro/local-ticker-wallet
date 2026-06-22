# [TUI][Crítico] Tela de Busca engole teclas de navegação (`1` `2` `3` `/` `q`)

**Severidade:** 🔴 Crítica
**Telas afetadas:** Busca (`busca de ativos`)
**Tipo:** Fluxo / Entrada de teclado

## Descrição

Na tela de Busca, o campo de texto é um input livre, mas as teclas globais de
navegação são interceptadas **antes** de chegarem ao campo. Como resultado, ao
digitar uma consulta:

- `1` → vai para a Carteira
- `2` ou `/` → (re)abre a Busca
- `3` → vai para o Livro de transações
- `q` → **encerra o aplicativo inteiro**

Na prática é **impossível pesquisar a maioria dos tickers da B3**, porque eles
quase sempre terminam em dígito: `VALE3`, `ITUB3`, `BBAS3`, `ABEV3`, `B3SA3`…
terminam em `3`; ETFs/FIIs/Units como `BOVA11`, `IVVB11`, `HGLG11` contêm `1`.
Além disso, qualquer `q` na consulta fecha o app sem aviso.

## Passos para reproduzir

1. Abra a TUI (`ltw tui`).
2. Pressione `2` para ir à Busca.
3. Digite `V`, `A`, `L`, `E` → o campo mostra `VALE` e a lista de resultados aparece.
4. Pressione `3` (querendo completar `VALE3`).

## Resultado esperado

O campo de busca passa a conter `VALE3` e a busca é refeita.

## Resultado obtido

A tela troca abruptamente para o **Livro de transações**; o `3` nunca é inserido.
De forma análoga, pressionar `q` enquanto digita encerra o app e volta ao shell.

## Evidência

- Vídeo: `bug_search_field_swallows_digit_and_q_keys.mp4` — digitar `3` pula para o Livro; digitar `q` fecha o app.
- Screenshot `screenshot_search_vale_typed.webp` (antes) e `screenshot_search_digit_jumped_to_ledger.webp` (depois de pressionar `3`).

## Causa raiz (suspeita)

Em `src/tui/screens/mod.rs`, `handle_key()` chama `handle_global_key()` **antes**
de despachar para o handler da tela ativa. `handle_global_key()` captura
`q`, `1`, `2`, `3` e `/` incondicionalmente, mesmo quando a Busca está ativa e
deveria estar em "modo de digitação".

```rust
// src/tui/screens/mod.rs
if let Some(outcome) = handle_global_key(app, data, code).await { return outcome; }
match app.screen { Screen::Search => search::handle_key(...).await, ... }
```

## Sugestão de correção

Quando `app.screen == Screen::Search`, não aplicar os atalhos globais de dígitos
/ `q` / `/` — encaminhar essas teclas ao `search::handle_key` como texto. Opções:

- Pular `handle_global_key` na tela de Busca (deixar apenas `Esc` para sair da Busca, como já existe), **ou**
- Introduzir um conceito de "input em foco" e só processar atalhos globais fora dele.

Considere também trocar o atalho de sair em telas de input (ex.: somente `Esc`,
ou `Ctrl+C`) para evitar que `q` feche o app durante a digitação.
