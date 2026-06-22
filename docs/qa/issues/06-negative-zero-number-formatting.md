# [TUI][Baixo] Formatação exibe `-0,00%` / `-R$ 0,00` (sinal de menos enganoso)

**Severidade:** 🔵 Baixa
**Telas afetadas:** Carteira, Detalhe, Prévia da Busca (qualquer % ou valor monetário)
**Tipo:** Valores / Visual

## Descrição

Valores negativos muito pequenos que arredondam para zero são exibidos com sinal
de menos, resultando em `-0,00%` e `-R$ 0,00`. O sinal é decidido a partir do
valor **original** (antes do arredondamento), mas a magnitude é arredondada para
`0,00`, gerando um "zero negativo" visualmente incorreto.

## Evidência (teste determinístico)

```
PROBE-FMT: pct(-0.001) = '-0,00%'  money(-0.004) = '-R$ 0,00'
```

## Causa raiz

`src/core/format.rs`:

```rust
pub fn format_pct(value: Decimal, locale: FormatLocale) -> String {
    let sign = if value > 0 { "+" } else if value < 0 { "-" } else { "" }; // usa valor não arredondado
    format!("{}{}%", sign, format_fixed(value.abs(), 2, locale))           // magnitude arredondada
}

pub fn format_money(value: Decimal, locale: FormatLocale) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    format!("{}{}{}", sign, locale.currency_prefix, format_fixed(value.abs(), 2, locale))
}
```

## Sugestão de correção

Determinar o sinal a partir do valor **já arredondado** na precisão de exibição.
Por exemplo, arredondar primeiro (`value.round_dp(places)`) e, se o resultado for
zero, não emitir sinal. Assim `-0.001%` vira `0,00%` e `-0.004` vira `R$ 0,00`.
