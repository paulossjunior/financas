# Contract — `get_startup_import_summary`

Tauri command. Devolve (e limpa) o resumo da importação automática executada no boot,
para o front mostrar um aviso discreto uma única vez.

## Frontend wrapper

```ts
// services/tauri.service.ts
getStartupImportSummary(): Promise<FolderImportSummary | null>
// invoke("get_startup_import_summary")
```

Chamado por `App.vue` no `onMounted`. Se `!= null`, mostrar toast/banner com as
contagens; ignorados/erros com detalhe opcional.

## Signature (backend)

```rust
#[tauri::command]
pub async fn get_startup_import_summary(
    cell: State<'_, Mutex<Option<FolderImportSummary>>>,
) -> Result<Option<FolderImportSummary>, String>
```

## Behavior

1. `take()` do `Option` na célula gerenciada (lê e limpa).
2. Retorna o resumo se o boot importou algo (ou registrou ignorados), senão `None`.

## Notas

- A célula é preenchida no `lib.rs setup` após `import_from_folder`, apenas quando
  `import_directory` está definido.
- Leitura única: chamadas subsequentes retornam `None` até o próximo boot.

## Acceptance (spec)

- US2-1: dados novos no painel após abrir (import ocorreu no boot).
- US2-2: resumo discreto exibido ao terminar a importação automática.
- US2-3: sem pasta configurada → célula vazia → `None` (nenhum aviso).
