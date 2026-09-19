# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## O sistema

Aplicação desktop (Tauri 2 + React + SQLite) de controle de saída e retorno de veículos leves do setor de Logística da usina. Roda por um `.exe` que mora numa pasta de rede Windows e é operada por um usuário de cada vez.

## Restrições arquiteturais

Estas cinco regras definem o projeto. Não as contorne, não proponha alternativas.

**1. Não existe API, servidor nem serviço de banco.** O `.exe` na pasta de rede é tudo que existe. Nada é gravado na máquina do usuário além do log. É isso que permite abrir de outra estação sem procedimento nenhum.

**2. O banco fica direto na pasta de rede.** Nunca implemente cópia local, thread de sincronismo, `VACUUM INTO` de volta ou fila de escrita. A durabilidade vem da configuração da conexão, obrigatória em toda abertura:

```sql
PRAGMA journal_mode = TRUNCATE;  -- WAL NÃO funciona sobre SMB
PRAGMA synchronous  = FULL;      -- flush forçado, anula cache SMB
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

Queda de rede no meio de uma escrita deixa o journal na pasta e o SQLite reverte sozinho na próxima abertura. Nunca apague um `controle.db-journal` órfão à mão, nem oriente o usuário a fazê-lo.

**3. Um usuário por vez.** Lock por handle exclusivo do Windows (`OpenOptions::share_mode(0)`) sobre `dados/controle.lock`, mais um `controle.lock.info` legível com heartbeat de 30 s. O lock é o que torna a restrição 2 segura: quase toda corrupção de SQLite em rede vem de escrita concorrente, e aqui ela não existe.

**4. Toda regra de negócio vive no Rust.** O React valida apenas para dar retorno imediato ao usuário, nunca como única barreira. Regra sem equivalente no backend é regra inexistente.

**5. Duas regras são garantidas pelo banco, por índice único parcial, não por código:**

```sql
CREATE UNIQUE INDEX ux_veiculo_em_uso ON saidas(veiculo_id)
  WHERE dt_chegada IS NULL AND excluida_em IS NULL;
CREATE UNIQUE INDEX ux_motorista_em_uso ON saidas(motorista_id)
  WHERE dt_chegada IS NULL AND excluida_em IS NULL;
```

Não substitua por `SELECT` prévio de verificação. O `SELECT` não impede a corrida, o índice sim.

## Convenções obrigatórias

- **Data-hora**: sempre `TEXT` no formato `'YYYY-MM-DD HH:MM'`, horário local. Nunca UTC, nunca epoch. Um fuso só, converter só criaria erro.
- **Vocabulário**: schema, comandos e structs em português (`saidas`, `motoristas`, `abrir_saida`, `encerrar_saida`). É o vocabulário do setor.
- **PRAGMA**: os quatro da restrição 2 rodam em toda conexão aberta. Não são padrão do SQLite e não são herdados entre conexões.
- **Escrita**: todo comando que escreve abre transação explícita e grava em `auditoria` dentro dela.
- **Erros**: zero `unwrap()` e `expect()` em caminho de execução. Erro vira `Result<T, ErroApp>` com campo `codigo` estável, e a mensagem chega à tela em português.
- **Exclusão**: sempre lógica, via `excluida_em` e `excluida_por`. Não existe `DELETE` de saída em nenhum caminho da aplicação.
- **Runtime offline**: fontes, ícones e bibliotecas embutidos no bundle. Nenhuma requisição de rede em runtime. As máquinas podem não ter internet.

## Estrutura do repositório

```
docs/SPEC.md            contrato de implementação (ver abaixo)
src/                    React + TypeScript (Vite). Só UI e validação de retorno imediato.
  telas/                uma tela por fluxo (saída, retorno, cadastros, relatórios)
  componentes/          reutilizáveis, sem regra de negócio
  api/                  wrappers de invoke(), um por comando Rust
  tipos/                espelho TS das structs serde
src-tauri/
  src/
    main.rs             bootstrap: lock, abertura do banco, registro dos comandos
    comandos/           #[tauri::command], um arquivo por agregado (saidas, motoristas, veiculos)
    dominio/            regras de negócio puras, testáveis sem banco
    db/                 conexão, PRAGMA, migrações, queries
    erro.rs             ErroApp, códigos estáveis, mapeamento de rusqlite::Error
    lock.rs             handle exclusivo e heartbeat do controle.lock.info
  migracoes/            SQL versionado, aplicado na abertura da conexão
dados/                  na pasta de rede, ao lado do .exe: controle.db, controle.lock, controle.lock.info
legado/                 HTML monolítico original. Referência de domínio, não é código vivo.
```

`dados/` não é versionado. Novas regras de negócio entram em `dominio/`, nunca em `comandos/`: comando orquestra transação, auditoria e tradução de erro, e nada mais.

## docs/SPEC.md

O contrato de implementação completo. Este arquivo não o resume nem o substitui. Abra o SPEC antes de:

- criar ou alterar tabela, índice ou migração (seção 2)
- escrever, assinar ou mudar um `#[tauri::command]`, ou acrescentar um código de `ErroApp` (seção 3)
- mexer em lock, heartbeat, abertura de sessão, backup ou rotação (seção 4)
- implementar uma validação de abertura, encerramento, cadastro, fusão ou exclusão (seção 6)
- escrever SQL de relatório ou mudar como duração e quilometragem são calculadas (seção 7)
- dar uma entrega por pronta, ou planejar a Fase 0 (seção 8)

Nunca copie trecho do SPEC para cá. Duas cópias divergem, e ninguém sabe qual vale.

## Comandos

```bash
npm install
npm run tauri dev        # app completo em modo dev (Vite + Rust com reload)
npm run tauri build      # bundle final em src-tauri/target/release
npm run build            # só o front: checagem de tipos + bundle Vite
npx tsc --noEmit         # checagem de tipos isolada

cargo test    --manifest-path src-tauri/Cargo.toml
cargo test    --manifest-path src-tauri/Cargo.toml nome_do_teste -- --exact --nocapture
cargo clippy  --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt     --manifest-path src-tauri/Cargo.toml
```

O bundle de produção é gerado no Windows (alvo WebView2). Build a partir do Linux serve para checagem de compilação, não para entrega.

## Armadilhas deste projeto

**Quando o SQLite devolver erro de I/O**, trate como servidor caído: mostre "Conexão com o servidor perdida" e ofereça reconectar, o que reabre a conexão e roda `integrity_check`. Nunca enfileire escritas em memória para aplicar depois, é a porta de entrada para divergência silenciosa.

**Quando vier `SQLITE_CONSTRAINT_UNIQUE`**, identifique o índice pelo nome na mensagem e mapeie para os códigos `VEICULO_EM_USO` ou `MOTORISTA_EM_USO`. Nunca deixe vazar como erro genérico de banco.

**Quando implementar cadastro rápido de condutor no formulário**, exija alerta de nome parecido (Levenshtein ≤ 2) antes de gravar, e garanta que a função de fundir cadastros exista. O sistema antigo acumulou 182 nomes de condutor para cerca de 70 pessoas reais porque o campo era texto livre.

**Quando mexer em qualquer coisa que leia saídas abertas**, lembre que "aberta" significa `dt_chegada IS NULL AND excluida_em IS NULL`. Filtro incompleto ressuscita registro excluído ou esconde veículo em uso.

## Definição de pronto

1. Compila sem warning (`npm run build` e `cargo build` limpos).
2. `cargo clippy --all-targets -- -D warnings` passa sem erro.
3. Existe teste cobrindo a regra alterada, e a suíte inteira passa.
