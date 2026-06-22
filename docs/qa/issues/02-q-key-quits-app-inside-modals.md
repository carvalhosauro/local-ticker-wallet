# [TUI][Crítico] `q` dentro dos modais fecha o app inteiro

**Severidade:** 🔴 Crítica
**Telas afetadas:** Overlay "Nova transação", Overlay "Confirmar exclusão"
**Tipo:** Fluxo / Entrada de teclado

## Descrição

Com um overlay (modal) aberto, pressionar a tecla `q` **encerra o aplicativo
inteiro**, em vez de:

- na **Nova transação**: inserir a letra `q` no campo de texto em foco (Ativo / Nota);
- na **Confirmar exclusão**: ser ignorada ou cancelar (hoje `Esc` cancela).

Consequências:

- Não é possível digitar tickers que contêm `q` (ex.: `QUAL3`) no campo Ativo.
- Não é possível escrever notas com a letra `q` (palavras comuns: "quantidade",
  "liquidez", "quero", "aquisição"…).
- Um `q` acidental no diálogo de exclusão fecha o app sem confirmar nem cancelar.

## Passos para reproduzir

1. Abra a TUI (`ltw tui`).
2. Pressione `a` para abrir o modal "Nova transação".
3. Navegue (`↓`) até o campo "Nota:".
4. Digite `a`, `b`, `c` → o campo mostra `abc` (letras normais funcionam).
5. Pressione `q`.

## Resultado esperado

O campo "Nota:" passa a mostrar `abcq`.

## Resultado obtido

O aplicativo inteiro fecha e retorna ao shell.

## Evidência

- Vídeo: `bug_q_key_quits_app_from_add_transaction_modal.mp4`.
- Screenshot `screenshot_addtx_note_before_q_quit.webp` (campo Nota com `abc`, imediatamente antes do `q` que fecha o app).

## Causa raiz

Em `src/tui/screens/mod.rs`, o ramo de overlay trata `q` como "sair" antes de
delegar ao handler do overlay:

```rust
// src/tui/screens/mod.rs — handle_key
if app.has_overlay() {
    if matches!(code, KeyCode::Char('q')) {
        return KeyOutcome::Quit;   // <- intercepta o 'q' mesmo dentro de um input
    }
    return crate::tui::overlays::handle_key(app, data, code).await;
}
```

Note ainda que apenas o `q` minúsculo é interceptado; `Q` (maiúsculo) chega ao
campo — comportamento inconsistente.

## Sugestão de correção

Não interceptar `q` quando há overlay aberto: encaminhar todas as teclas (exceto,
opcionalmente, `Esc`) ao `overlays::handle_key`. O modal "Nova transação" já
trata `Esc` para fechar; o de exclusão também. A saída global por `q` deve valer
apenas nas telas principais sem input de texto em foco.
