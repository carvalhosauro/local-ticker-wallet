# [TUI][Médio] `o` (ordenar por score) não desfaz a ordenação e não reposiciona a seleção

**Severidade:** 🟡 Média
**Telas afetadas:** Carteira
**Tipo:** Fluxo / Valores

## Descrição

A tecla `o` alterna `app.sort_by_score`. A cada frame o loop principal chama
`sort_positions(&mut data.positions, app.sort_by_score)`, que ordena por score
**apenas quando ligado**; quando desligado é um no-op. Portanto:

1. Ligar `o` ordena por score (desc) — ok.
2. Desligar `o` **não restaura** a ordem original (alfabética): a lista
   permanece ordenada por score, pois o no-op não reordena. Só um `r` (refresh,
   que refaz o fetch) traz a ordem alfabética de volta.

Além disso, a ordenação reordena `data.positions` mas não remapeia
`app.portfolio_selected` (que é um índice posicional): após ordenar, a linha
destacada passa a apontar para outro ativo silenciosamente.

## Passos para reproduzir

1. Na Carteira, observe a ordem alfabética (ABEV3, B3SA3, …).
2. Pressione `o` → ordena por score (desc).
3. Pressione `o` novamente (esperando voltar à ordem anterior).

## Resultado esperado

Voltar à ordenação original (alfabética). E, ao ordenar, manter selecionado o
mesmo ativo.

## Resultado obtido

A lista continua ordenada por score. A seleção destaca um ativo diferente do
que estava selecionado antes da ordenação.

## Causa raiz

- `src/tui/mod.rs` (loop): `client::sort_positions(&mut data.positions, app.sort_by_score);`
- `src/tui/client.rs`:

```rust
pub fn sort_positions(rows: &mut [PositionRow], by_score: bool) {
    if by_score { rows.sort_by(|a, b| b.score.cmp(&a.score)...); }
    // else: não faz nada -> ordem anterior preservada
}
```

A seleção (`portfolio_selected`) não é convertida/preservada por símbolo ao
reordenar.

## Sugestão de correção

- Guardar a ordem base (alfabética) e reaplicá-la quando `sort_by_score` for
  desligado (ou reordenar explicitamente por símbolo no ramo `else`).
- Preservar a seleção por símbolo: antes de ordenar, lembrar o símbolo
  selecionado e reposicionar o índice após a ordenação.
