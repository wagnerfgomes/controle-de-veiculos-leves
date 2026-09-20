# Plano de implementação

Plano de execução do `docs/SPEC.md`. O SPEC é o contrato e não é repetido aqui: este arquivo diz **em que ordem** o contrato é cumprido, **o que trava o quê** e **o que ainda precisa de decisão humana**.

Nada deste plano foi executado. O repositório está no estado descrito em "Estado atual".

---

## A restrição que define a ordem

A ordem natural seria validar a arquitetura de rede primeiro, porque a Fase 0 do SPEC é gate: reprovando, a seção 4 inteira vira cópia local com sincronismo.

Só que a Fase 0 exige caminho de rede definitivo e autorização para rodar um `.exe` de lá, e **isso não se consegue antes de apresentar o sistema funcionando**. Ninguém libera pasta e assina liberação de executável por causa de um projeto que existe só em markdown. A burocracia é sequencial e não adianta brigar com ela.

Então o plano inverte: **construir até a apresentação, apresentar, e só então validar a rede.**

```
Fase 0a  sonda de política e estresse     (Windows do setor, pasta já acessível)
         ═══ em paralelo, pode começar hoje, não depende de nenhuma outra ═══

Fase 1  scaffold e config.toml          (Linux)
Fase 2  fundação do banco               (Linux)
Fase 3  domínio puro                    (Linux)
Fase 4  sessão e lock local             (Linux)
Fase 5  comandos                        (Linux)
Fase 6  telas                           (Linux)
Fase 7  relatórios manuais              (Linux)
Fase 8  demonstração no notebook          ◄── destrava tudo que vem depois
        │
        └─► aprovação, caminho de rede definitivo e liberação da TI
                │
Fase 0b  item zero no UNC e lock em duas máquinas
                │
Fase 9  endurecimento de rede           (depende do veredito)
Fase 10 build Windows, empacotamento, homologação e piloto
```

### O preço dessa inversão, e como pagá-lo barato

Construir o app inteiro antes de validar a rede significa que uma reprovação na Fase 0 chega depois de muito código escrito. É risco real e não dá para eliminar, só para conter em três movimentos:

1. **Isolar tudo que depende de rede em três arquivos**: `src-tauri/src/db/conexao.rs`, `src-tauri/src/lock.rs` e `src-tauri/src/sessao.rs`. Reprovando a Fase 0, o desenho de cópia local com sincronismo é reescrito nesses três e em mais nada. Schema, domínio, comandos, telas e relatórios não sabem de onde vem o arquivo do banco, e não podem saber.
2. **Rodar o teste de estresse antes da autorização formal.** O `martelo` da Fase 0 não precisa que o `.exe` more no compartilhamento: precisa que o **banco** esteja lá. Existe uma pasta de rede com escrita já disponível, então o CLI pode rodar do disco local ou de pendrive contra ela. É a Fase 0a, e derruba cedo o maior risco do projeto.
3. **Não implementar os dois desenhos.** O SPEC proíbe, e código morto de sincronismo é o que reaparece ligado por engano seis meses depois.

### Definição de pronto

Vale para toda fase que produza código, vinda do `CLAUDE.md`: compila sem warning, `cargo clippy --all-targets -- -D warnings` limpo, teste cobrindo a regra alterada quando ela estiver na tabela de testes mínimos do SPEC, nenhum `unwrap()` ou `expect()` novo em caminho de execução.

---

## Fase 1 — scaffold e `config.toml`

**Onde roda:** Linux. **Depende de:** nada.

Tauri 2 + React 18 + TypeScript 5 + Vite 5, na estrutura exata que o `CLAUDE.md` contrata. Sem template cheio de exemplo: as pastas nascem vazias e um comando `ping` prova que o `invoke()` atravessa.

Arquivos: `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/main.tsx`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `config.toml.exemplo`.

`config.toml` fica ao lado do executável, nunca compilado no binário, com `caminho_dados` e `modo`. Em desenvolvimento o modo é `producao` apontando para uma pasta local, e o código não sabe a diferença.

**Pronto quando:** `npm run tauri dev` abre a janela, o `ping` responde, `npm run build` e `cargo build` limpos.

---

## Fase 2 — fundação do banco

**Onde roda:** Linux. **Depende de:** Fase 1.

| Arquivo | Conteúdo |
| --- | --- |
| `src-tauri/migracoes/001_inicial.sql` | seção 2 do SPEC, literal, com os dois índices únicos parciais. Única migração: a base nasce vazia e para em `user_version = 1` |
| `src-tauri/src/db/conexao.rs` | abertura mais os quatro PRAGMA, em toda conexão, sem exceção |
| `src-tauri/src/db/migracoes.rs` | leitura do `PRAGMA user_version`, backup antes de cada migração, aplicação em ordem |
| `src-tauri/src/erro.rs` | `ErroApp` com os nove códigos da seção 3 e o mapeamento de `rusqlite::Error` |
| `src-tauri/src/db/auditoria.rs` | gravação em `auditoria` dentro da transação do chamador |

O ponto delicado é o mapeamento de `SQLITE_CONSTRAINT_UNIQUE`: identificar o índice **pelo nome na mensagem** e traduzir para `VEICULO_EM_USO` ou `MOTORISTA_EM_USO`. Nunca vazar como `BANCO`. Isso é testado, não confiado.

**Pronto quando:** teste de migração de schema vazio até v1 passa, e um teste lê de volta os quatro PRAGMA confirmando que estão ativos.

---

## Fase 3 — domínio puro

**Onde roda:** Linux. **Depende de:** Fase 1. Independe de banco e de Tauri.

Funções puras em `src-tauri/src/dominio/`, testáveis sem abrir conexão. Melhor relação entre risco de bug e custo de teste do projeto inteiro, por isso vem antes dos comandos.

| Arquivo | Regra |
| --- | --- |
| `nome.rs` | `nome_norm`: trim, maiúsculas, NFD, descarte de marcas combinantes, colapso de espaços. Mesmo tratamento para `frota` |
| `semelhanca.rs` | Levenshtein ≤ 2 sobre `nome_norm`; ≤ 1 dígito de diferença em `frota` |
| `duracao.rs` | horas entre duas datas ISO, sem adivinhação de virada de meia-noite; limiar de 14 h |
| `validacao.rs` | as tabelas da seção 6: abertura, encerramento, cadastro, fusão, exclusão |
| `datahora.rs` | formatação e parsing de `YYYY-MM-DD HH:MM` local. Nenhuma função aceita UTC nem epoch |

Dependências novas: `unicode-normalization` e `strsim`. Pequenas, sem dependência transitiva relevante, sem acesso a rede.

**Pronto quando:** passam os testes mínimos que não tocam o banco: `"José  Carlos"` colidindo com `"JOSE CARLOS"`, `ANGELO JUNIOR` contra `ANJELO JUNIOR` devolvendo `SEMELHANTE`, `22:40 → 05:30` dando 6,83 h sem aviso, e duração acima de 14 h devolvendo `DURACAO_LONGA`.

---

## Fase 4 — sessão e lock local

**Onde roda:** Linux para compilar, Windows para valer. **Depende de:** Fase 2.

| Arquivo | Conteúdo |
| --- | --- |
| `src-tauri/src/lock.rs` | trait `Lock`, `LockWindows` com `share_mode(0)`, `LockDesenvolvimento` com flock, heartbeat de 30 s, `controle.lock.info`, tomada de posse |
| `src-tauri/src/sessao.rs` | sequência de abertura da seção 4 e o fechamento |
| `src-tauri/src/comandos/sessao.rs` | `estado_sessao`, `tomar_posse_lock` |

Fica **fora** desta fase o que só faz sentido com rede de verdade: modo degradado, `reconectar()` e a reação a erro de I/O. Vão para a Fase 9, depois do veredito da Fase 0. Implementar antes seria escrever contra uma seção 4 que ainda pode ser reescrita.

O stub de Linux é obrigado a se denunciar: aviso no log, indicador na barra de status e `compile_error!` em release, como o SPEC agora contrata. Nenhum teste de concorrência conta se rodado em Linux.

**Pronto quando:** em Windows, duas instâncias na mesma máquina produzem a tela de bloqueio na segunda, e matar a primeira pelo Gerenciador de Tarefas libera o lock sem intervenção. Isso já é demonstrável na apresentação.

---

## Fase 5 — comandos

**Onde roda:** Linux. **Depende de:** Fases 2, 3 e 4.

Um arquivo por agregado em `src-tauri/src/comandos/`: `motoristas.rs`, `veiculos.rs`, `saidas.rs`. Comando orquestra transação, auditoria e tradução de erro, e nada mais. Regra nova vai para `dominio/`.

Ordem: CRUD de motoristas, CRUD de veículos, fusão, abertura de saída, encerramento, edição e exclusão, listagens e KPIs.

Como o cadastro é inteiramente manual, o CRUD deixa de ser tela de manutenção e vira a porta de entrada do sistema: base nova, ninguém cadastrado, e a primeira coisa que alguém faz é cadastrar frota e funcionário. Precisa ser rápido de usar e difícil de sujar.

Dois pontos concentram o risco:

- **`confirmado`**, que sustenta o cadastro rápido. Em `false`, procura semelhante e devolve `SEMELHANTE` com os candidatos **sem gravar nada**. Gravar e perguntar depois é exatamente o bug que produziu 182 nomes para 70 pessoas.
- **`fundir_*`**, em transação: reaponta saídas, grava o registro removido inteiro em JSON na auditoria, apaga o duplicado. Recusa se qualquer dos dois tiver viagem aberta, senão o índice único estoura no meio.

`excluir_saida` agora exige `motivo: String`, recusado vazio ou com menos de 3 caracteres.

**Pronto quando:** `VEICULO_EM_USO` e `MOTORISTA_EM_USO` voltam com o código certo e não como `BANCO`, exclusão lógica libera o índice único, e `fundir_motoristas` preserva a contagem de saídas.

---

## Fase 6 — telas

**Onde roda:** Linux. **Depende de:** Fase 5.

`src/api/` com um wrapper de `invoke()` por comando, `src/tipos/` espelhando as structs serde, `src/telas/` com Painel, Nova saída, Encerrar saída, Histórico, Gestão e Bloqueio.

Visual herdado do HTML atual: paleta azul e verde, `Barlow Condensed` nos títulos, `Barlow` no corpo, densidade alta, cor por turno. **Fontes e ícones entram no bundle como arquivo local**, nunca por CDN: as máquinas podem não ter internet, e fonte que não carrega quebra a densidade da tela inteira.

Ordem: Painel primeiro, que responde à pergunta que o setor faz o tempo todo. Depois Nova saída, Encerrar, Gestão, Histórico, Bloqueio.

O React valida para dar retorno imediato e nunca é a única barreira.

**Pronto quando:** os itens de "Dados" da seção 8 passam clicando na interface, sem console aberto.

---

## Fase 7 — relatórios manuais

**Onde roda:** Linux. **Depende de:** Fases 5 e 6.

| Arquivo | Conteúdo |
| --- | --- |
| `src-tauri/src/db/relatorios.rs` | as consultas da seção 7, literais |
| `src-tauri/src/dominio/relatorio.rs` | cabeçalho obrigatório e formatação |

Só a emissão manual: `gerar_relatorio` e `previa_relatorio`. A varredura automática de meses pendentes fica na Fase 9, junto com o resto do que só existe em produção.

Está aqui, antes da apresentação, porque relatório é o que justifica o sistema para quem decide. Painel bonito impressiona o operador; número consolidado impressiona quem assina a liberação.

CSV com BOM UTF-8 e separador `;`. PDF pela impressão do WebView. `:ate` exclusivo em toda consulta. Viagem aberta conta como viagem e não soma hora.

**Pronto quando:** os dois relatórios batem com contagem manual de uma semana de dados fictícios, o cabeçalho mostra o percentual de viagens sem chegada, e o CSV abre no Excel com acentuação correta.

---

## Fase 8 — modo demonstração e apresentação

**Onde roda:** onde a apresentação acontecer. **Depende de:** Fases 4 a 7.

É o marco que destrava o projeto inteiro. Entregável não é código, é autorização.

- `modo = "demonstracao"` no `config.toml`, conforme a seção 4 do SPEC: banco em `.\dados-demo` ao lado do executável, recriado do zero a cada abertura.
- **Gerador de base fictícia**: veículos, motoristas e viagens espalhadas por dois meses, umas abertas e outras fechadas, uma passando de 24 h para a linha vermelha aparecer, alguns nomes propositalmente parecidos para demonstrar o alerta de duplicata. Tela vazia não apresenta nada, e dado fictício mal feito apresenta pior ainda.
- Faixa permanente **MODO DEMONSTRAÇÃO** na interface.
- Backup, rotação e relatórios automáticos desligados. Emissão manual ligada.
- Lock ativo: duas instâncias na mesma máquina mostram a tela de bloqueio.

Roteiro sugerido de apresentação, na ordem em que convence: Painel com carros na rua, abrir uma viagem em 20 segundos, encerrar com um clique, tentar abrir o mesmo veículo duas vezes e mostrar a recusa, cadastrar um nome parecido e mostrar o alerta, emitir o relatório do mês, abrir duas instâncias e mostrar o bloqueio.

**Pronto quando:** a demo roda do início ao fim sem tocar em rede, em máquina que não seja a sua, e sai dela um caminho de pasta e uma resposta da TI sobre executável.

---

## Fase 0 do SPEC — validação da arquitetura de rede

**Onde roda:** duas máquinas Windows do setor, contra o compartilhamento real.
**Depende de:** o acesso que a Fase 8 destrava. O item `martelo` pode e deve acontecer antes, ver "o preço dessa inversão".

Continua sendo a Fase 0 do SPEC: o que mudou foi quando ela cabe no calendário, não o que ela vale. **Nada da Fase 9 começa antes do veredito.**

Binário descartável em Rust **sem Tauri**, um CLI de umas 200 linhas com `rusqlite` e o lock. Sem Tauri porque um CLI cross-compila daqui do Arch com `x86_64-pc-windows-gnu` mais `mingw-w64`, e assim o teste não exige montar ambiente de desenvolvimento em máquina do setor.

| Modo | O que faz |
| --- | --- |
| `martelo` | grava em loop no banco da rede, uma transação por segundo, imprimindo o número da última confirmada |
| `checar` | abre o banco, roda `integrity_check` e compara a contagem com o último número confirmado |
| `lock` | tenta adquirir o lock, informa sucesso ou falha, segura o handle até o Enter |
| `tempo` | mede abertura da conexão mais os quatro PRAGMA, 20 repetições |

Roteiro, que é o da seção 8 do SPEC, agora com o item zero na frente:

0. Copiar um `.exe` não assinado para o caminho UNC e abri-lo nas máquinas do setor. Bloqueou, pare: o modelo de entrega inteiro depende disso.
1. `martelo` rodando, puxar o cabo no meio da escrita, reconectar, `checar`. **20 vezes, variando o momento da interrupção, 20 aprovações.** Planilha com momento e resultado de cada uma.
2. `lock` nas duas máquinas ao mesmo tempo: exatamente uma obtém.
3. Matar o processo detentor: a outra obtém sem intervenção.
4. `tempo` com o antivírus ativo. Registrar a média.

**Se o item 1 reprovar:** a seção 4 do SPEC é reescrita para cópia local com sincronismo antes de qualquer linha da Fase 9. Fases 1 a 8 sobrevivem, e o estrago fica contido em `conexao.rs`, `lock.rs` e `sessao.rs`.

---

## Fase 9 — endurecimento de rede

**Onde roda:** Windows, contra o compartilhamento real. **Depende do veredito da Fase 0.**

O que só existe em produção e não tinha como ser testado antes:

- Modo degradado: heartbeat falhando 3 vezes, escrita recusada com `SESSAO_PERDIDA`, saída por `reconectar()`.
- Erro de I/O mapeado para `CONEXAO_PERDIDA`, com a tela oferecendo reconectar.
- `integrity_check` na abertura, com o desvio para `backups\corrompidos\`.
- `VACUUM INTO` a cada 4 h, ao fechar e antes de migração, mais a rotação 48 h / 30 d / 12 m.
- Varredura de relatórios pendentes na abertura, gerando **todos** os meses fechados que faltarem, com piso em `config.data_corte`.

**Pronto quando:** rotação com 40 arquivos sintéticos mantém a política, varredura com 3 meses pendentes gera os 3 na ordem, varredura em base nova não volta além de `data_corte`, e **um backup é restaurado numa cópia e conferido**. Backup nunca restaurado não é backup.

---

## Fase 10 — empacotamento, homologação e piloto

**Onde roda:** Windows. **Depende de:** todas.

1. Build de produção no Windows, alvo WebView2. Build no Linux serve para checagem de compilação, não é entrega.
2. `.exe` e `config.toml` na pasta de rede, com `modo = "producao"`. `dados\`, `backups\` e `relatorios\` criados.
3. Percorrer a seção 8 do SPEC inteira com alguém do setor, sem ler código.
4. **Piloto de uma semana** registrando nos dois sistemas em paralelo e comparando os números ao fim. É o que dá confiança para desligar o HTML antigo.

---

## Decisões tomadas

| Decisão | Resolução |
| --- | --- |
| Cadastro de frota e funcionário | **Manual, CRUD.** Cadastra quem não existe e atribui a viagem. `002_seed_frotas.sql` removido do SPEC, schema para em `user_version = 1` |
| `motivo` em `excluir_saida` | **Obrigatório**, `String`, recusado vazio ou com menos de 3 caracteres |
| Extração do HTML legado | **Cortada.** O `legado/` fica como referência de domínio e nada mais |
| Dependências `unicode-normalization` e `strsim` | Aceitas |
| Ordem de execução | Invertida: apresentação antes da validação de rede, porque a liberação depende dela |

## Decisões ainda em aberto

| # | Decisão | Trava |
| --- | --- | --- |
| 1 | Onde a demo roda: seu notebook Linux, ou uma máquina Windows do setor | Define se a Fase 8 precisa de um build Windows antecipado, puxando parte da Fase 10 para antes |
| 2 | Caminho de rede onde já existe escrita hoje, para o `martelo` antecipado | Só afeta quando o risco da Fase 0 cai, não bloqueia nenhuma fase |

---

## Riscos

| Risco | Impacto | O que reduz |
| --- | --- | --- |
| Fase 0 reprova depois do app pronto | Seção 4 reescrita com o projeto quase inteiro construído | Rede confinada a três arquivos, e o `martelo` rodado cedo contra qualquer pasta com escrita |
| TI bloqueia `.exe` em caminho UNC | Modelo de entrega inviável | Item zero da Fase 0, e a apresentação é justamente onde se descobre com quem falar |
| Apresentação com base vazia ou dado fictício ruim | Não convence, e sem convencer não há acesso | Gerador de base fictícia é entregável da Fase 8, não improviso da véspera |
| Stub de lock silencioso em Linux | Descobrir em produção que nunca houve lock | Aviso, indicador na tela e `compile_error!` em release, já no SPEC |
| Cadastro manual sem rede de segurança | Repetir os 182 nomes para 70 pessoas | `SEMELHANTE` antes de gravar, indicador de duplicatas na Gestão, fusão funcionando desde a Fase 5 |
| Antivírus tornando a abertura lenta demais | Reprova o critério de 5 segundos | Medido na Fase 0, com tempo para negociar exclusão de pasta com a TI |

---

## Estado atual

`main`, árvore limpa. Existe: `CLAUDE.md`, `docs/SPEC.md` (797 linhas), `docs/PLANO.md`, `legado/gerenciamento_veiculos_leves_sja_2026-08-18.html` e `.gitignore`.

Não existe ainda: scaffold, migrações, código Rust, código React, `config.toml`, pasta de rede definida.

Ambiente verificado: cargo 1.93.1, rustc 1.93.1, node 22.22.0, npm 11.10.1, `webkit2gtk-4.1` presente. Único alvo Rust instalado é `x86_64-unknown-linux-gnu`; a Fase 0 exige acrescentar `x86_64-pc-windows-gnu` mais `mingw-w64`, e a Fase 10 exige build em máquina Windows.
