# [TUI][Médio] Painel de prévia da Busca fica preso em "Carregando cotação…" após falha

**Severidade:** 🟡 Média
**Telas afetadas:** Busca (painel "Prévia")
**Tipo:** Visual / Estado

## Descrição

Quando a cotação de prévia falha (ex.: provedor indisponível), o painel "Prévia"
continua exibindo **"Carregando cotação…"** indefinidamente, embora não haja mais
nenhuma requisição em andamento. O único sinal do erro é um toast temporário
("Cotação indisponível: …") que desaparece após alguns segundos, deixando o
painel num estado de "carregando" permanente e enganoso.

## Passos para reproduzir

1. Abra a Busca e digite um termo que retorne resultados.
2. Selecione um resultado cuja cotação o provedor não consiga fornecer
   (ou com o provedor de cotação fora do ar).

## Resultado esperado

Após a falha, o painel mostra uma mensagem de erro/placeholder (ex.: "Cotação
indisponível") em vez de continuar "carregando".

## Resultado obtido

O painel fica permanentemente em "Carregando cotação…".

## Evidência

Screenshot `screenshot_search_preview_stuck_loading.webp`: painel direito em
"Carregando cotação…" enquanto a barra de status (vermelha) mostra
"Cotação indisponível: provider brapi failed".

## Causa raiz

Em `src/tui/screens/search.rs`, `render_preview_panel`:

```rust
let lines = if let Some(p) = preview {
    /* mostra a prévia */
} else if app.search_preview_pending || results.get(selected).is_some() {
    vec![Line::from(b.search_preview_loading)]   // <- cai aqui sempre que há seleção
} else {
    vec![Line::from(b.search_preview_select)]
};
```

Após a falha, `tick_preview` zera `search_preview = None` e
`search_preview_pending = false`, mas como `results.get(selected).is_some()`
continua verdadeiro, a UI sempre exibe "carregando".

## Sugestão de correção

Diferenciar os estados "pendente", "erro" e "vazio". Por exemplo, guardar um
`search_preview_error: Option<String>` (ou um enum de estado da prévia) e exibir
a mensagem de erro no painel quando a última tentativa falhar, em vez de reusar a
mensagem de carregamento.
