# Plano de implementação

Plano de execução do `docs/SPEC.md`. O SPEC é o contrato e não é repetido aqui: este arquivo diz **em que ordem** o contrato é cumprido, **o que trava o quê** e **o que precisa de decisão humana antes de virar código**.

Nada deste plano foi executado. O repositório está no estado descrito em "Estado atual".

---

## Como ler

Cada fase traz: objetivo, do que depende, onde roda (Linux de desenvolvimento ou Windows do setor), entregáveis por arquivo, critério de pronto e os commits previstos. Fase sem critério de pronto verificável não é fase, é intenção.

A **Definição de pronto** do `CLAUDE.md` vale para toda fase que produza código: compila sem warning, `cargo clippy --all-targets -- -D warnings` limpo, teste cobrindo a regra alterada quando ela estiver na tabela de testes mínimos do SPEC, nenhum `unwrap()` ou `expect()` novo em caminho de execução.

---

## Ordem de execução e o gate da Fase 0

A Fase 0 do SPEC é gate bloqueante: reprovando o teste de estresse, a seção 4 inteira é substituída por cópia local com sincronismo. Ela precisa de duas máquinas Windows contra o compartilhamento real e **não roda na máquina de desenvolvimento**.

Por isso a ordem não é linear. O que não depende da seção 4 é feito em paralelo, enquanto a Fase 0 é agendada com o setor:

```
Fase 0  (Windows, com o setor)  ─────────── gate ───────────┐
                                                            │
Fase 1  scaffold + extração do legado    (Linux) ──┐        │
Fase 2  fundação do banco                (Linux) ──┤        │
Fase 3  domínio puro                     (Linux) ──┘        │
                                                            ▼
                                            Fase 4  sessão e lock
                                            Fase 5  comandos
                                            Fase 6  telas
                                            Fase 7  backup e relatórios
                                            Fase 8  empacotamento e piloto
```

Fases 1, 2 e 3 sobrevivem inteiras a uma reprovação da Fase 0: schema, normalização de nome, Levenshtein, cálculo de duração e regras de validação não mudam se o banco passar a ser cópia local. O que morre é a Fase 4. Começar por elas é o que evita jogar trabalho fora.

**Não comece a Fase 4 antes do veredito da Fase 0.** Não implemente os dois desenhos "por segurança": o SPEC proíbe explicitamente, e código morto de sincronismo é exatamente o que reaparece ligado por engano seis meses depois.

---

## Fase 0 — validação da arquitetura de rede

**Onde roda:** duas máquinas Windows do setor, contra o compartilhamento real.
**Depende de:** acesso ao caminho UNC definitivo e autorização da TI para rodar um `.exe` de lá.
**Entregável:** binário descartável, que não entra no repositório como código vivo.

Protótipo em Rust **sem Tauri**: um CLI de umas 200 linhas com `rusqlite` e o lock por `share_mode(0)`. Sem Tauri porque um CLI cross-compila de Arch para Windows com `x86_64-pc-windows-gnu` mais `mingw-w64`, e assim a Fase 0 não exige montar ambiente de desenvolvimento em máquina do setor. `rusqlite` com feature `bundled` compila o SQLite em C pelo mingw sem problema.

Modos do binário:

| Modo | O que faz |
| --- | --- |
| `martelo` | grava em loop no banco da rede, uma transação por segundo, imprimindo o número da última gravação confirmada |
| `checar` | abre o banco, roda `integrity_check`, compara a contagem com o último número confirmado pelo `martelo` |
| `lock` | tenta adquirir o lock, informa sucesso ou falha e segura o handle até o Enter |
| `tempo` | mede o tempo de abertura da conexão mais os quatro PRAGMA, em 20 repetições |

Roteiro, que é o da seção 8 do SPEC:

1. `martelo` rodando, puxar o cabo de rede no meio da escrita, reconectar, `checar`. **20 vezes, variando o momento da interrupção. 20 aprovações, sem exceção.** Registrar em planilha o momento da interrupção e o resultado.
2. `lock` nas duas máquinas ao mesmo tempo: exatamente uma obtém.
3. Matar o processo que detém o lock pelo Gerenciador de Tarefas: a outra máquina obtém sem intervenção manual.
4. `tempo` com o antivírus ativo. Registrar a média.

**Critério de pronto:** planilha com as 20 execuções do item 1, todas aprovadas, mais os resultados de 2, 3 e 4.

**Se o item 1 reprovar:** pare tudo. A seção 4 do SPEC é reescrita para cópia local com sincronismo antes de qualquer outra linha de código da Fase 4 em diante. Fases 1, 2 e 3 seguem válidas.

---

## Fase 1 — scaffold e extração do legado

**Onde roda:** Linux. **Depende de:** nada.

### 1a. Scaffold

Tauri 2 + React 18 + TypeScript 5 + Vite 5, na estrutura exata que o `CLAUDE.md` contrata. Nada de template cheio de exemplo: o scaffold nasce com as pastas vazias e um comando `ping` só para provar que o `invoke()` atravessa.

Arquivos: `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/main.tsx`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `config.toml.exemplo`.

O caminho da pasta de rede vem de `config.toml` ao lado do executável, nunca compilado. Em desenvolvimento aponta para uma pasta local; o código não sabe a diferença.

**Critério de pronto:** `npm run tauri dev` abre a janela e o `ping` responde. `npm run build` e `cargo build` limpos.

### 1b. Extração do legado

Script descartável em `legado/extracao/extrair.mjs`, lendo o HTML de 7.454 linhas e produzindo um relatório. Objetivo **não** é migrar dado, que o SPEC já descartou (base nova), é responder a quatro perguntas antes de congelar o schema:

- Quais campos do array JavaScript estão realmente preenchidos, e em que percentual? Campo que ninguém preencheu em dois anos não merece coluna.
- Qual a frequência real de cada frota? É o que confirma ou corrige a lista de 18 códigos do `002_seed_frotas.sql`, que o próprio SPEC manda conferir com o setor antes de rodar.
- Quais são os 182 nomes de condutor, agrupados por distância de Levenshtein ≤ 2? Dimensiona o problema de deduplicação e vira material de conferência com o setor.
- Quais destinos e atividades mais aparecem? Alimentam a sugestão de destino da tela de Nova saída.

**Critério de pronto:** `legado/extracao/relatorio.md` gerado, com as quatro respostas, e a lista de frotas levada ao setor para confirmação.

**Commits:** `chore: adiciona scaffold tauri react vite`, `chore: adiciona extrator do html legado`, `docs: registra relatorio de extracao do legado`.

---

## Fase 2 — fundação do banco

**Onde roda:** Linux. **Depende de:** Fase 1a. A migração `002` depende da confirmação das frotas (Fase 1b).

Entregáveis:

| Arquivo | Conteúdo |
| --- | --- |
| `src-tauri/migracoes/001_inicial.sql` | seção 2 do SPEC, literal, incluindo os dois índices únicos parciais |
| `src-tauri/migracoes/002_seed_frotas.sql` | seed das frotas, só após confirmação do setor |
| `src-tauri/src/db/conexao.rs` | abertura mais os quatro PRAGMA, em toda conexão |
| `src-tauri/src/db/migracoes.rs` | leitura de `PRAGMA user_version`, backup antes de cada migração, aplicação em ordem |
| `src-tauri/src/erro.rs` | `ErroApp` com os nove códigos da seção 3 e o mapeamento de `rusqlite::Error` |
| `src-tauri/src/db/auditoria.rs` | gravação em `auditoria` dentro da transação do chamador |

O ponto delicado é o mapeamento de `SQLITE_CONSTRAINT_UNIQUE`: identificar o índice **pelo nome na mensagem** e traduzir para `VEICULO_EM_USO` ou `MOTORISTA_EM_USO`. Nunca deixar vazar como `BANCO`. Esse mapeamento é testado, não confiado.

**Critério de pronto:** teste "migração de schema vazio até v2" passa, terminando com `user_version = 2` e seed aplicado. Teste de conexão confirma os quatro PRAGMA ativos, lendo de volta cada um.

**Commits:** um por arquivo de migração, um para conexão e PRAGMA, um para `ErroApp`, um para auditoria.

---

## Fase 3 — domínio puro

**Onde roda:** Linux. **Depende de:** Fase 1a. Independe de banco e de Tauri.

Tudo em `src-tauri/src/dominio/`, funções puras, testáveis sem abrir conexão. É o módulo com melhor relação entre risco de bug e custo de teste, e por isso vem antes dos comandos.

| Arquivo | Regra |
| --- | --- |
| `nome.rs` | `nome_norm`: trim, maiúsculas, decomposição NFD, descarte de marcas combinantes, colapso de espaços. Mesmo tratamento para `frota` |
| `semelhanca.rs` | Levenshtein ≤ 2 sobre `nome_norm`; ≤ 1 dígito de diferença em `frota` |
| `duracao.rs` | horas entre duas datas ISO, sem adivinhação de virada de meia-noite; limiar de 14 h |
| `validacao.rs` | as tabelas da seção 6: abertura, encerramento, cadastro, fusão, exclusão |
| `datahora.rs` | formatação e parsing de `YYYY-MM-DD HH:MM` local, e só isso. Nenhuma função aceita UTC nem epoch |

Duas dependências novas, que precisam de aval: `unicode-normalization` (NFD) e `strsim` (Levenshtein). Ambas sem dependência transitiva relevante e sem acesso a rede. Escrever Levenshtein à mão também resolve, são 20 linhas, mas é código para manter sem ganho.

**Critério de pronto:** os testes da tabela de testes mínimos que não tocam o banco passam, especificamente `"José  Carlos"` colidindo com `"JOSE CARLOS"`, `ANGELO JUNIOR` contra `ANJELO JUNIOR` devolvendo `SEMELHANTE`, `22:40 → 05:30` dando 6,83 h sem aviso, e duração acima de 14 h devolvendo `DURACAO_LONGA`.

---

## Fase 4 — sessão e lock

**Onde roda:** Linux para compilar, Windows para valer. **Depende do veredito da Fase 0.**

| Arquivo | Conteúdo |
| --- | --- |
| `src-tauri/src/lock.rs` | handle exclusivo, heartbeat de 30 s, `controle.lock.info`, tomada de posse |
| `src-tauri/src/sessao.rs` | sequência de abertura da seção 4, em ordem, e o fechamento |
| `src-tauri/src/comandos/sessao.rs` | `estado_sessao`, `tomar_posse_lock`, `reconectar`, `forcar_backup` |

O lock é Windows puro (`std::os::windows::fs::OpenOptionsExt::share_mode(0)`) e não compila em Linux. Como a máquina de desenvolvimento é Arch, o módulo precisa de um contrato:

```
trait Lock  →  adquirir, heartbeat, liberar, info
  #[cfg(windows)]  LockWindows   share_mode(0), o que vale em produção
  #[cfg(unix)]     LockDesenvolvimento   flock advisory, só para o dev rodar
```

O stub de Linux existe para o `npm run tauri dev` funcionar na máquina de desenvolvimento, e precisa gritar isso: log de aviso na inicialização e recusa de rodar em build de release. Um stub silencioso é como se descobre em produção que o lock nunca existiu.

O SPEC não define a struct `EstadoSessao`, que os comandos devolvem. Definir aqui, minimamente com: usuário Windows, máquina, início da sessão, estado da conexão (normal ou degradado), último backup e nível de schema.

**Critério de pronto:** em duas máquinas Windows, a segunda mostra a tela de bloqueio com nome e hora da primeira. Matar o processo detentor libera o lock sem intervenção. Heartbeat falhando 3 vezes leva ao modo degradado, e `reconectar()` sai dele.

---

## Fase 5 — comandos

**Onde roda:** Linux. **Depende de:** Fases 2, 3 e 4.

Um arquivo por agregado em `src-tauri/src/comandos/`: `motoristas.rs`, `veiculos.rs`, `saidas.rs`. Comando orquestra transação, auditoria e tradução de erro, e nada mais: regra nova vai para `dominio/`.

Ordem sugerida, da menor para a maior dependência: gestão de motoristas, gestão de veículos, fusão, abertura de saída, encerramento, edição e exclusão, listagens e KPIs.

Dois pontos que concentram o risco:

- **`confirmado`**, o parâmetro que sustenta o cadastro rápido. Em `false`, procura semelhante e devolve `SEMELHANTE` com os candidatos **sem gravar nada**. Gravar e perguntar depois seria o mesmo bug que produziu 182 nomes para 70 pessoas.
- **`fundir_*`**, em transação: reaponta saídas, grava o registro removido inteiro em JSON na auditoria, apaga o duplicado. Recusa se qualquer dos dois tiver viagem aberta, senão o índice único estoura no meio da transação.

O SPEC também não define `Kpis` nem `RelatorioDados`. Definir junto com os comandos que os devolvem.

**Critério de pronto:** testes de `VEICULO_EM_USO` e `MOTORISTA_EM_USO` devolvendo o código certo e não `BANCO`, exclusão lógica liberando o índice único, e `fundir_motoristas` preservando a contagem de saídas.

---

## Fase 6 — telas

**Onde roda:** Linux. **Depende de:** Fase 5.

`src/api/` com um wrapper de `invoke()` por comando, `src/tipos/` espelhando as structs serde, `src/telas/` com uma tela por fluxo: Painel, Nova saída, Encerrar saída, Histórico, Gestão e Bloqueio.

O visual herda o do HTML atual, paleta azul e verde, `Barlow Condensed` nos títulos e `Barlow` no corpo. **As fontes entram no bundle como arquivo local**, nunca por CDN: as máquinas podem não ter internet, e fonte que não carrega quebra a densidade da tela inteira. Mesma regra para ícones.

Ordem: Painel primeiro, porque responde à pergunta que o setor faz o tempo todo e permite homologar cedo com gente de verdade. Depois Nova saída, Encerrar, Histórico, Gestão, Bloqueio.

O React valida para dar retorno imediato e nunca é a única barreira. Toda regra da seção 6 já está no Rust ao chegar aqui.

**Critério de pronto:** os itens de "Dados" da seção 8 passam clicando na interface, sem console aberto.

---

## Fase 7 — backup e relatórios

**Onde roda:** Linux para lógica, Windows para conferir tempo sobre SMB. **Depende de:** Fases 4 e 5.

| Arquivo | Conteúdo |
| --- | --- |
| `src-tauri/src/db/backup.rs` | `VACUUM INTO` a cada 4 h, ao fechar e antes de migração, mais a rotação 48 h / 30 d / 12 m |
| `src-tauri/src/db/relatorios.rs` | as consultas da seção 7, literais |
| `src-tauri/src/dominio/relatorio.rs` | cabeçalho obrigatório, varredura de meses pendentes, formatação |

A varredura de meses pendentes é a parte que já escondeu um bug uma vez: gera **todos** os meses fechados que faltarem, do mais antigo ao mais novo, com piso em `config.data_corte` quando `ultimo_relatorio_periodo` estiver vazio.

CSV com BOM UTF-8 e separador `;`. PDF pela impressão do WebView. `:ate` é exclusivo em toda consulta.

**Critério de pronto:** rotação com 40 arquivos sintéticos mantém exatamente a política da tabela, varredura com 3 meses pendentes gera os 3 na ordem, varredura em base nova não volta além do mês de `data_corte`, e **um backup é restaurado numa cópia e conferido**. Backup nunca restaurado não é backup.

---

## Fase 8 — empacotamento, homologação e piloto

**Onde roda:** Windows. **Depende de:** todas.

1. Build de produção no Windows, alvo WebView2. Build a partir do Linux serve para checagem de compilação e não é entrega.
2. `.exe` e `config.toml` na pasta de rede, `dados\`, `backups\` e `relatorios\` criados.
3. Percorrer a seção 8 do SPEC inteira, com alguém do setor, sem ler código.
4. **Piloto de uma semana** registrando nos dois sistemas em paralelo e comparando os números ao fim. É o que dá confiança para desligar o HTML antigo.

**Critério de pronto:** todos os itens da seção 8 marcados, e a semana de piloto fechada com os números batendo.

---

## Decisões pendentes

Nenhuma destas é minha para tomar, e três delas travam fase.

| # | Decisão | Trava | Recomendação |
| --- | --- | --- | --- |
| 1 | `motivo` obrigatório em `excluir_saida` | Fase 5 | **Tornar obrigatório.** É uma linha na assinatura. Exclusão sem motivo é a que ninguém explica três meses depois, e o Histórico já reserva espaço para exibi-lo |
| 2 | Confirmar as 18 frotas do seed com o setor | Fase 2 (`002`) | A lista saiu de frequência no HTML antigo, não de inventário. O próprio SPEC manda conferir |
| 3 | Caminho UNC definitivo e autorização da TI | Fase 0 | Sem isso a Fase 0 não acontece, e sem Fase 0 nada depois dela começa |
| 4 | Dependências novas: `unicode-normalization` e `strsim` | Fase 3 | Aceitar. Alternativa é escrever Levenshtein à mão, 20 linhas para manter sem ganho |
| 5 | Pastas fora da estrutura contratada: `legado/extracao/`, `src-tauri/src/db/relatorios.rs`, `src-tauri/src/dominio/relatorio.rs` | Fases 1b e 7 | Aceitar. O `CLAUDE.md` contrata a estrutura e estas três são extensões coerentes com ela, não desvios |

---

## Alterações propostas ao SPEC

Propostas, não aplicadas. O SPEC segue como está até você decidir.

1. **Acrescentar à Fase 0 um item zero:** rodar um `.exe` não assinado a partir do caminho UNC nas máquinas do setor, antes do teste de estresse. Se a política de TI ou o SmartScreen bloquearem, o modelo de entrega inteiro morre e o teste de estresse nem precisa acontecer. Hoje isso só aparece na seção "Ambiente", que é homologação final, tarde demais para descobrir.
2. **Definir as três structs que a seção 3 devolve e nunca descreve:** `EstadoSessao`, `Kpis` e `RelatorioDados`. As demais saem do schema, estas não.
3. **Contratar o comportamento do lock fora do Windows.** A seção 4 só descreve Windows, e a máquina de desenvolvimento é Linux. Sem contrato, cada sessão improvisa um stub diferente.
4. **`motivo` obrigatório**, se a decisão 1 for essa.

---

## Riscos

| Risco | Impacto | O que reduz |
| --- | --- | --- |
| Fase 0 reprova | Seção 4 reescrita, Fases 4 a 8 refeitas | Ordem do plano: 1, 2 e 3 sobrevivem inteiras. Fazer a Fase 0 cedo, em paralelo, não no fim |
| TI bloqueia `.exe` em caminho UNC | Projeto inteiro inviável no formato atual | Item zero proposto para a Fase 0 |
| Lock em Linux com stub silencioso | Descobrir em produção que nunca houve lock | Aviso na inicialização e recusa em build de release |
| Lista de 18 frotas errada | Seed sujo numa base nova, cadastro manual para desfazer | Confirmação com o setor antes de rodar `002` |
| Antivírus tornando a abertura lenta demais | Reprova o critério de 5 segundos | Medido na Fase 0, com tempo de sobra para negociar exclusão de pasta com a TI |
| Cadastro rápido sem a rede de segurança | Repetir os 182 nomes para 70 pessoas | `SEMELHANTE` antes de gravar, indicador de duplicatas na Gestão e fusão funcionando desde a Fase 5 |

---

## Estado atual

`main`, árvore limpa, oito commits. Existe: `CLAUDE.md`, `docs/SPEC.md` (703 linhas, 8 seções), `docs/PLANO.md` (este arquivo), `legado/gerenciamento_veiculos_leves_sja_2026-08-18.html` e `.gitignore`.

Não existe ainda: scaffold, migrações, código Rust, código React, `config.toml`, pasta de rede definida.

Ambiente de desenvolvimento verificado: cargo 1.93.1, rustc 1.93.1, node 22.22.0, npm 11.10.1, `webkit2gtk-4.1` presente. Único alvo Rust instalado é `x86_64-unknown-linux-gnu`; a Fase 0 exige acrescentar `x86_64-pc-windows-gnu` mais `mingw-w64`, e a Fase 8 exige build em máquina Windows.
