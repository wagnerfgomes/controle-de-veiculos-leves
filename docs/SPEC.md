# Especificação técnica — Controle de Veículos Leves

Contrato de implementação. Escrito para ser executável por um desenvolvedor ou por um agente de código sem precisar de contexto externo.

---

## 1. Escopo e stack

### Decisões fixadas

| Item | Decisão |
| --- | --- |
| Acesso ao banco | **direto na pasta de rede**, sem cópia local |
| Validação desse acesso | teste de estresse na Fase 0, antes de qualquer funcionalidade |
| Dados históricos | não migrados; base nova |
| Cadastro de frota e motorista | **manual**, por CRUD na tela de Gestão ou pelo formulário de abertura de viagem. Sem seed: a base nasce vazia |
| Bloqueio de viagem aberta | veículo **e** motorista |
| Hodômetro | presente desde o início, opcional no preenchimento |
| Modo de consulta somente leitura | não existe como funcionalidade; relatório é emitido pelo operador. O modo degradado da seção 4 é outra coisa: estado de falha, não de uso |

### Stack

```
Tauri        2.x
Rust         edition 2021
rusqlite     0.32+ (feature "bundled")
serde/serde_json
chrono       (datas locais)
tokio        (tarefas de fundo: heartbeat, backup)
React        18 + TypeScript 5
Vite         5
```

Sem dependência de rede em runtime. Fontes e ícones embutidos no bundle, nada de CDN, porque as máquinas podem não ter internet.

### Convenções obrigatórias

- **Data-hora:** sempre `TEXT` no formato `YYYY-MM-DD HH:MM`, horário local. Nunca UTC, nunca timestamp numérico. Todo o setor opera num fuso só, e converter só criaria chance de erro.
- **Toda regra de negócio vive no Rust.** O React valida para orientar o usuário; o comando revalida porque a tela pode ser contornada.
- **Todo comando que escreve abre transação explícita** e grava em `auditoria` dentro dela.
- **Toda conexão executa os quatro PRAGMA** da seção 4. Não são padrão do SQLite e não são herdados entre conexões.
- **Nenhum `unwrap()` ou `expect()` em caminho de execução.** Erro vira `Result<T, ErroApp>` e chega à tela com mensagem em português.
- **Nomes em português** no schema e nos comandos, para acompanhar o vocabulário do setor.

### Layout de pastas

```
\\servidor\logistica\veiculos-leves\        (rede — é tudo)
├── ControleVeiculos.exe
├── config.toml
├── dados\  controle.db · controle.lock · controle.lock.info
├── backups\  (e backups\corrompidos\)
└── relatorios\AAAA-MM\

%LOCALAPPDATA%\ControleVeiculos\             (máquina)
└── app.log        ← só log, nenhum dado
```

O caminho da pasta de rede vai em `config.toml` ao lado do `.exe`, não compilado no binário.

---

## 2. Schema

Arquivos em `src-tauri/migracoes/`, aplicados em ordem na abertura. `PRAGMA user_version` guarda o nível aplicado; backup obrigatório antes de qualquer migração.

### `001_inicial.sql`

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE motoristas (
  id        INTEGER PRIMARY KEY,
  nome      TEXT NOT NULL,
  nome_norm TEXT NOT NULL UNIQUE,
  matricula TEXT UNIQUE,
  setor     TEXT,
  ativo     INTEGER NOT NULL DEFAULT 1,
  criado_em TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE veiculos (
  id     INTEGER PRIMARY KEY,
  frota  TEXT NOT NULL UNIQUE,
  placa  TEXT UNIQUE,
  modelo TEXT,
  marca  TEXT,
  ano    INTEGER,
  tipo   TEXT,
  hodometro_atual INTEGER,
  ativo  INTEGER NOT NULL DEFAULT 1,
  observacao TEXT,
  criado_em TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE saidas (
  id           INTEGER PRIMARY KEY,
  veiculo_id   INTEGER NOT NULL REFERENCES veiculos(id),
  motorista_id INTEGER NOT NULL REFERENCES motoristas(id),
  turno        TEXT NOT NULL CHECK (turno IN ('A','B','C')),
  destino      TEXT,
  atividade    TEXT NOT NULL,
  dt_saida     TEXT NOT NULL,
  dt_chegada   TEXT,
  hodometro_saida   INTEGER,
  hodometro_chegada INTEGER,
  chegada_manual    INTEGER NOT NULL DEFAULT 0,
  observacao   TEXT,
  excluida_em  TEXT,
  excluida_por TEXT,
  excluida_motivo TEXT,
  criado_em    TEXT NOT NULL DEFAULT (datetime('now','localtime')),
  atualizado_em TEXT,
  CHECK (dt_chegada IS NULL OR dt_chegada > dt_saida),
  CHECK (hodometro_chegada IS NULL OR hodometro_saida IS NULL
         OR hodometro_chegada >= hodometro_saida)
);

-- As duas regras de ouro, garantidas pelo banco:
CREATE UNIQUE INDEX ux_veiculo_em_uso ON saidas(veiculo_id)
  WHERE dt_chegada IS NULL AND excluida_em IS NULL;
CREATE UNIQUE INDEX ux_motorista_em_uso ON saidas(motorista_id)
  WHERE dt_chegada IS NULL AND excluida_em IS NULL;

CREATE INDEX ix_saidas_periodo   ON saidas(dt_saida);
CREATE INDEX ix_saidas_veiculo   ON saidas(veiculo_id, dt_saida);
CREATE INDEX ix_saidas_motorista ON saidas(motorista_id, dt_saida);

CREATE TABLE auditoria (
  id INTEGER PRIMARY KEY,
  entidade TEXT NOT NULL,
  entidade_id INTEGER,
  acao TEXT NOT NULL,
  antes TEXT, depois TEXT,
  usuario_windows TEXT, maquina TEXT,
  em TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE config (chave TEXT PRIMARY KEY, valor TEXT);

INSERT INTO config (chave, valor) VALUES
  ('ultimo_backup_em', ''),
  ('ultimo_relatorio_periodo', ''),
  ('data_corte', date('now','localtime'));

PRAGMA user_version = 1;
```

### Sem seed

A base nasce vazia. Frota e motorista são cadastrados à mão, por CRUD, e a viagem é atribuída depois. Não existe migração de seed, e o schema para em `user_version = 1`.

Importar a lista de frotas do sistema antigo importaria também o lixo dele, que é justamente o que este projeto existe para não repetir.

### A tabela `config`

Três chaves, todas escritas pelo Rust:

| Chave | Função |
| --- | --- |
| `ultimo_backup_em` | data-hora do último `VACUUM INTO` concluído, exibida na barra de status |
| `ultimo_relatorio_periodo` | último mês fechado (`AAAA-MM`) cujo relatório já foi gerado. Vazio numa base nova |
| `data_corte` | data da primeira abertura do sistema. É o piso da varredura de relatórios pendentes da seção 4: numa base nova `ultimo_relatorio_periodo` está vazio, e sem piso o app não sabe até onde voltar |

O nível de schema aplicado mora em `PRAGMA user_version`, e só lá. Não duplique esse número numa chave de `config`. As duas cópias divergem na primeira migração escrita com pressa, e `user_version` é a que o SQLite honra.

### `nome_norm`

Gerado em Rust, não em SQL: `trim` → maiúsculas → remove acentos (decomposição NFD, descarta marcas combinantes) → colapsa espaços múltiplos. `"José  Carlos "` e `"JOSE CARLOS"` produzem a mesma chave, e o `UNIQUE` recusa o segundo cadastro.

O mesmo tratamento se aplica a `frota`: maiúsculas e sem espaços.

### Regra de `hodometro_atual`

Atualizado no encerramento da viagem quando `hodometro_chegada` for informado e maior que o valor atual. Serve só para pré-preencher a próxima saída, nunca é usado em cálculo de relatório, porque pode estar defasado.

---

## 3. Comandos Tauri

### Erro padrão

Todo comando devolve `Result<T, ErroApp>`. O React trata pelo `codigo`, nunca pela mensagem.

```rust
#[derive(serde::Serialize)]
pub struct ErroApp {
    pub codigo: String,   // VEICULO_EM_USO, MOTORISTA_EM_USO, ...
    pub mensagem: String, // texto em português para a tela
    pub detalhe: Option<String>,
}
```

| Código | Quando |
| --- | --- |
| `VEICULO_EM_USO` | o veículo já tem viagem aberta |
| `MOTORISTA_EM_USO` | o motorista já tem viagem aberta |
| `DUPLICADO` | `nome_norm` ou `frota` já existe |
| `SEMELHANTE` | cadastro parecido encontrado; exige confirmação |
| `NAO_ENCONTRADO` | id inexistente |
| `VALIDACAO` | regra de campo violada; `detalhe` diz qual |
| `BANCO` | falha de SQLite |
| `SESSAO_PERDIDA` | o lock foi perdido durante a sessão; o app está em modo degradado e recusa escrita |
| `CONEXAO_PERDIDA` | erro de I/O: o servidor de arquivos caiu |

### Gestão

```rust
listar_motoristas(incluir_inativos: bool) -> Vec<Motorista>
buscar_motoristas(termo: String) -> Vec<Motorista>   // combo com busca
criar_motorista(nome: String, matricula: Option<String>,
                setor: Option<String>,
                confirmado: bool) -> Motorista
atualizar_motorista(id: i64, ...) -> Motorista
inativar_motorista(id: i64) -> ()
fundir_motoristas(manter_id: i64, remover_id: i64) -> u32  // saídas reapontadas
possiveis_duplicatas_motoristas() -> Vec<(Motorista, Motorista)>

listar_veiculos(incluir_inativos: bool) -> Vec<Veiculo>
listar_veiculos_disponiveis() -> Vec<VeiculoDisponibilidade>
buscar_veiculos(termo: String) -> Vec<Veiculo>
criar_veiculo(frota: String, placa: Option<String>, ...,
              confirmado: bool) -> Veiculo
atualizar_veiculo(id: i64, ...) -> Veiculo
inativar_veiculo(id: i64) -> ()
fundir_veiculos(manter_id: i64, remover_id: i64) -> u32
possiveis_duplicatas_veiculos() -> Vec<(Veiculo, Veiculo)>
```

**`confirmado`** é o parâmetro que sustenta o cadastro rápido. Em `false`, o comando procura cadastro semelhante (Levenshtein ≤ 2 sobre `nome_norm`, ou ≤ 1 dígito de diferença em `frota`) e, se achar, devolve `SEMELHANTE` com os candidatos em `detalhe`, sem gravar nada. A tela pergunta, e só então reenvia com `confirmado: true`.

**`fundir_*`** roda numa transação: reaponta as saídas para `manter_id`, registra em `auditoria` o registro removido inteiro em JSON, apaga o duplicado. Recusa se algum dos dois tiver viagem aberta, senão o índice único estoura no meio.

```rust
struct VeiculoDisponibilidade {
    veiculo: Veiculo,
    disponivel: bool,
    motorista_atual: Option<String>,  // quem está com o carro
    desde: Option<String>,
    destino_atual: Option<String>,
}
```

O combo mostra os ocupados esmaecidos com essas informações em vez de escondê-los: quem registra precisa saber a quem recorrer.

### Saídas

```rust
abrir_saida(veiculo_id: i64, motorista_id: i64, turno: String,
            destino: Option<String>, atividade: String,
            dt_saida: Option<String>,        // None = agora
            hodometro_saida: Option<i64>,
            observacao: Option<String>,
            confirmado_avisos: bool) -> Saida

encerrar_saida(id: i64,
               dt_chegada: Option<String>,   // None = agora, chegada_manual = 0
               hodometro_chegada: Option<i64>,
               confirmado_avisos: bool) -> Saida

editar_saida(id: i64, ...) -> Saida
excluir_saida(id: i64, motivo: String) -> ()   // lógica; grava excluida_motivo

listar_saidas_abertas() -> Vec<SaidaAberta>
listar_saidas(filtro: FiltroSaidas) -> Vec<Saida>
kpis(filtro: FiltroSaidas) -> Kpis
```

`abrir_saida` em transação única:

1. Valida campos (seção 6).
2. `SELECT` de viagem aberta para o veículo e para o motorista.
3. `INSERT`. O índice único parcial é a barreira final. Se estourar `SQLITE_CONSTRAINT_UNIQUE`, mapear para `VEICULO_EM_USO` ou `MOTORISTA_EM_USO` pelo nome do índice, nunca deixar vazar como `BANCO`.
4. `INSERT` em `auditoria`.
5. `COMMIT`. Com `synchronous = FULL`, o commit só retorna depois do flush, não há etapa posterior de sincronismo.

`encerrar_saida` com duração acima de 14 h devolve `VALIDACAO` com `detalhe: "DURACAO_LONGA:16h20"`. A tela confirma com o usuário e reenvia com `confirmado_avisos: true`. É o que separa o turno legítimo de monitor do `08:00 → 07:00` digitado errado.

```rust
struct SaidaAberta {
    id: i64, frota: String, motorista: String,
    destino: Option<String>, atividade: String,
    dt_saida: String,
    horas_decorridas: f64,
    alerta: bool,        // > 24 h
}

struct FiltroSaidas {
    de: Option<String>, ate: Option<String>,   // 'YYYY-MM-DD'
    veiculo_id: Option<i64>, motorista_id: Option<i64>,
    turnos: Vec<String>,
    somente_abertas: bool,
    incluir_excluidas: bool,
}
```

### Sessão e relatórios

```rust
estado_sessao() -> EstadoSessao
tomar_posse_lock(confirmacao: String) -> ()   // exige "CONFIRMAR"
forcar_backup() -> String                     // caminho gerado
reconectar() -> ()                            // após erro de I/O

gerar_relatorio(tipo: TipoRelatorio, filtro: FiltroSaidas,
                formato: Formato) -> String   // caminho do arquivo
previa_relatorio(tipo: TipoRelatorio, filtro: FiltroSaidas) -> RelatorioDados
```

`TipoRelatorio` = `UsoVeiculo | UsoMotorista`. `Formato` = `Html | Pdf | Csv`.

### Structs que não saem do schema

As demais saem direto das tabelas. Estas três não, e por isso ficam definidas aqui:

```rust
struct EstadoSessao {
    usuario_windows: String,
    maquina: String,
    inicio: String,                  // 'YYYY-MM-DD HH:MM'
    conexao: Conexao,                // Normal | Degradada
    ultimo_backup: Option<String>,
    user_version: i64,               // lido do PRAGMA, nunca de config
    caminho_dados: String,           // o que a barra de status mostra
}

struct Kpis {
    viagens: u32,
    abertas: u32,
    condutores_distintos: u32,
    frotas_distintas: u32,
    duracao_media_horas: Option<f64>,   // None quando nenhuma viagem fechou
    por_turno: [u32; 3],                // A, B, C
}

struct RelatorioDados {
    cabecalho: CabecalhoRelatorio,
    linhas: Vec<LinhaRelatorio>,
}

struct CabecalhoRelatorio {
    de: String, ate: String,            // :ate exclusivo
    viagens: u32,
    sem_chegada: u32,            sem_chegada_pct: f64,
    chegadas_manuais: u32,       chegadas_manuais_pct: f64,
    hodometro_completo: u32,     hodometro_completo_pct: f64,
    emitido_em: String, emitido_por: String,
}

struct LinhaRelatorio {
    rotulo: String,                     // frota e modelo, ou nome e matrícula
    viagens: u32,
    abertas: u32,
    horas_totais: f64,
    horas_media: Option<f64>,
    chegadas_manuais: u32,
    km: Option<i64>,                    // só em UsoVeiculo
    veiculos_distintos: Option<u32>,    // só em UsoMotorista
    frotas: Option<String>,             // só em UsoMotorista
}
```

`duracao_media_horas` e `horas_media` são `Option` de propósito: média de zero viagens fechadas é ausência de dado, não zero. Zero na tela afirma que os carros rodaram sem gastar tempo.

---

## 4. Sessão

### Abertura, em ordem

1. Ler `config.toml`, resolver o caminho da pasta de rede. Inacessível → tela de erro com o caminho tentado.
2. **Adquirir o lock.** Falhou → ler `controle.lock.info` e mostrar a tela de bloqueio. Fim.
3. Abrir a conexão **sobre o arquivo da rede** e configurar:

   ```sql
   PRAGMA journal_mode = TRUNCATE;  -- WAL NÃO funciona sobre SMB
   PRAGMA synchronous  = FULL;      -- flush forçado, anula cache SMB
   PRAGMA foreign_keys = ON;
   PRAGMA busy_timeout = 5000;
   ```

   O banco não existe → criar e rodar as migrações.
4. `PRAGMA integrity_check`. Falhou → bloquear escrita, mover o arquivo para `backups\corrompidos\` e oferecer restauração.
5. Aplicar migrações pendentes, com backup antes de cada uma.
6. Gerar relatórios pendentes (abaixo).
7. Iniciar as tarefas de fundo: heartbeat 30 s e backup 4 h.

### Lock

```rust
use std::os::windows::fs::OpenOptionsExt;

let lock = OpenOptions::new()
    .write(true).create(true)
    .share_mode(0)                 // FILE_SHARE_NONE
    .open(caminho_rede.join("dados/controle.lock"))?;
```

O handle fica vivo na struct de estado do app pela sessão inteira. O Windows o libera sozinho se o processo morrer, é o que evita lock órfão em travamento, `Fim de tarefa` ou queda de energia da estação.

`controle.lock.info` é reescrito a cada 30 s, em arquivo separado porque `share_mode(0)` bloqueia até a leitura:

```json
{"usuario_windows":"DOMINIO\\a.lopes","maquina":"PC-LOG-02",
 "pid":8124,"inicio":"2026-09-18T08:12:44","heartbeat":"2026-09-18T09:47:14"}
```

**Tomada de posse** só é oferecida com heartbeat parado há mais de 10 minutos. Exige digitar `CONFIRMAR`, registra em `auditoria` com `acao = 'TOMADA_LOCK'` e grava quem tomou de quem. Sem esse atraso, vira o botão que todo mundo aperta.

Se o heartbeat falhar 3 vezes seguidas durante a sessão, o app entra em **modo degradado**: recusa toda escrita com `SESSAO_PERDIDA` e avisa que a rede caiu e o lock pode ter sido perdido. Como o banco é o arquivo da rede, nesse estado não há como gravar de qualquer forma; o aviso serve para o usuário não continuar digitando achando que está registrando.

Modo degradado é estado de falha, não funcionalidade. Não o confunda com a linha "modo de consulta somente leitura" da seção 1, que não existe: ninguém abre o sistema nesse modo, só cai nele. Sair exige `reconectar()`, que reabre a conexão, readquire o lock e roda `integrity_check`.

### O lock fora do Windows

`share_mode(0)` é API do Windows e não compila em Linux, que é onde o desenvolvimento acontece. O módulo expõe um trait e duas implementações:

```rust
trait Lock {
    fn adquirir(caminho: &Path) -> Result<Self, ErroApp> where Self: Sized;
    fn heartbeat(&self)         -> Result<(), ErroApp>;
    fn info(caminho: &Path)     -> Result<Option<InfoLock>, ErroApp>;
    fn liberar(self);
}

#[cfg(windows)] LockWindows          // share_mode(0). É o que vale em produção.
#[cfg(unix)]    LockDesenvolvimento  // flock advisory, só para o dev rodar
```

O stub de Linux é obrigado a se denunciar: aviso no log na inicialização, indicador na barra de status e `compile_error!` em build de release sob `#[cfg(all(unix, not(debug_assertions)))]`. Stub silencioso é como se descobre em produção que o lock nunca existiu.

`flock` advisory não é equivalente ao handle exclusivo: protege contra outra instância na mesma máquina e nada além disso. Nenhum teste de concorrência conta se rodado em Linux; os itens de lock da seção 8 só valem em Windows, contra o compartilhamento real.

### Por que não há sincronismo

O banco é o arquivo da rede. **Não existe cópia local, thread de sincronismo, `VACUUM INTO` de volta, debounce nem fila de escrita. Nada disso deve ser implementado.**

A durabilidade vem de `synchronous = FULL`: cada commit força o flush antes de retornar. Uma queda de rede no meio de uma escrita deixa o journal na pasta, e o SQLite faz o rollback sozinho na próxima abertura. **Nunca apague um `controle.db-journal` órfão à mão**, é exatamente o que permite a reversão.

Perdendo a conexão com o servidor, o SQLite devolve erro de I/O. Mapear para `CONEXAO_PERDIDA`; a tela mostra "Conexão com o servidor perdida, o último registro pode não ter sido salvo" e oferece reconectar, que reabre a conexão e roda `integrity_check`. **Não enfileire escritas em memória para aplicar depois:** é a porta de entrada para divergência silenciosa.

**Se a Fase 0 reprovar**, esta seção inteira é substituída pelo desenho de cópia local, e aí sim entram sincronismo, rename atômico e recuperação de cópia órfã. Não implemente os dois.

### Backup

`VACUUM INTO 'backups\controle_AAAA-MM-DD_HHMM.db'` a cada 4 h de sessão, ao fechar, e antes de cada migração. `VACUUM INTO` produz cópia consistente por construção; cópia byte a byte de um arquivo sendo escrito pode capturar páginas de dois estados.

Rotação, executada após cada backup:

| Faixa | Mantém |
| --- | --- |
| Últimas 48 h | todos |
| Últimos 30 dias | o último de cada dia |
| Últimos 12 meses | o último de cada mês |
| Anteriores | descarta |

### Relatórios automáticos

Na abertura, varrer do mês seguinte a `config.ultimo_relatorio_periodo` até o mês anterior ao corrente, gerando os dois tipos em `relatorios\AAAA-MM\` para cada mês fechado que faltar e atualizando a chave a cada um. Numa base nova a chave está vazia: o piso da varredura passa a ser o mês de `config.data_corte`, senão o app tentaria voltar indefinidamente. É o que cobre a queda de energia: mesmo que ninguém tenha fechado o sistema, e mesmo que o sistema fique meses parado, os relatórios pendentes saem no próximo login.

Ao fechar, gerar o relatório do período corrente por cima do anterior do mesmo mês.

Em ambos os casos o app apenas mostra um toast com o caminho. Abrir uma janela de relatório na cara de quem entrou para registrar uma saída atrapalha.

### Fechamento

Backup → relatório → fecha a conexão → libera o handle do lock → remove `controle.lock.info`.

Não há cópia local a limpar. Se o app for encerrado abruptamente, o Windows libera o handle do lock sozinho e o SQLite reverte o journal na abertura seguinte; os dois casos se resolvem sem intervenção.

---

## 5. Telas

Cinco telas. O visual herda o do HTML atual: paleta azul/verde, `Barlow Condensed` nos títulos, `Barlow` no corpo, densidade alta, código de cor por turno. A equipe já está treinada nesse layout; mudá-lo custaria adaptação sem ganho.

### Painel (inicial)

Responde à pergunta que o setor faz o tempo todo: quais carros estão na rua e há quanto tempo.

- **Viagens abertas**, mais antigas no topo, com tempo decorrido. Acima de 24 h, linha em vermelho. Botão **Encerrar** em cada uma.
- **Veículos disponíveis**, em grade compacta.
- **KPIs** do topo, iguais aos de hoje: total, condutores, frotas, duração média, distribuição A/B/C.
- Botão grande **Nova saída**.
- Barra de status: usuário, estado da conexão com o servidor, último backup.

### Nova saída

Modal. Ordem dos campos na sequência em que a informação chega por rádio:

| Campo | Tipo | Obrigatório |
| --- | --- | --- |
| Veículo | combo com busca, indisponíveis esmaecidos | sim |
| Motorista | combo com busca | sim |
| Turno | A / B / C, pré-selecionado pelo horário | sim |
| Data/hora de saída | preenchido com agora, editável | sim |
| Destino | texto com sugestão dos anteriores | não |
| Atividade | texto | sim |
| Hodômetro | número, pré-preenchido com `hodometro_atual` | não |
| Observação | texto | não |

Digitar algo que não existe nos combos oferece **"Cadastrar 907015"**, abrindo mini-formulário sem sair do modal. Encontrando cadastro semelhante, o diálogo pergunta: *"Já existe ANJELO JUNIOR. É a mesma pessoa?"* com **Sim, usar o existente** / **Não, é outra pessoa**.

O turno é pré-selecionado pelo horário e permanece editável. A base antiga mostra turno A às 03:00 e C às 12:00: a regra real depende da escala, não do relógio, então derivar seria errado.

### Encerrar saída

Modal pequeno: dados da viagem em leitura, **Encerrar agora** como ação principal, e um link "informar outro horário" que revela o campo de data/hora. Campo de hodômetro de chegada, opcional.

O caminho rápido tem que ser o de um clique: a alta demanda é o que produz as chegadas não registradas.

### Histórico

Tabela igual à de hoje, com filtros por período, turno, veículo e motorista. Editar e excluir por linha. Exportar CSV. Viagens excluídas aparecem só com o filtro **"mostrar excluídas"** ligado, riscadas e com o motivo da exclusão.

### Gestão

Abas Motoristas e Veículos. CRUD, busca, inativar, e a ação de **fundir cadastros**: seleciona dois, o sistema mostra quantas saídas serão reapontadas, confirma.

Um indicador de **possíveis duplicatas** no topo, listando pares com distância ≤ 2, torna a limpeza um hábito em vez de um mutirão. É a rede de segurança do cadastro rápido.

### Tela de bloqueio

Quando o lock está tomado. Sem menu, sem contorno:

> **Sistema em uso**
> a.lopes · PC-LOG-02 · desde 08:12
> Procure essa pessoa antes de tentar novamente.
> [Tentar novamente]

O botão de tomada de posse só aparece com heartbeat parado há mais de 10 minutos, e vem acompanhado do aviso de risco de corrupção.

---

## 6. Validações

Todas aplicadas no Rust. O React repete as de campo para dar retorno imediato, mas nunca é a única barreira.

### Abertura

| Caso | Comportamento |
| --- | --- |
| Veículo com viagem aberta | recusa `VEICULO_EM_USO`, informa quem está com ele |
| Motorista com viagem aberta | recusa `MOTORISTA_EM_USO` |
| Veículo ou motorista inativo | recusa `VALIDACAO` |
| Atividade vazia | recusa `VALIDACAO` |
| Turno fora de A/B/C | recusa `VALIDACAO` |
| `dt_saida` no futuro além de 5 min | recusa `VALIDACAO` |
| `dt_saida` com mais de 7 dias | confirmação |
| Hodômetro menor que `hodometro_atual` | confirmação ("registrado 45.200, informado 44.900") |

### Encerramento

| Caso | Comportamento |
| --- | --- |
| Chegada ≤ saída | recusa (`CHECK` do banco também barra) |
| Chegada no futuro além de 5 min | recusa |
| Duração > 14 h | confirmação, com a duração no texto |
| Hodômetro de chegada < saída | recusa (`CHECK` do banco também barra) |
| Diferença de hodômetro > 1.000 km | confirmação |
| Viagem já encerrada | recusa `VALIDACAO` |

O limite de 14 h vem da base antiga: existem turnos legítimos de monitor com 10 a 12 h, então o alerta não pode disparar antes disso.

### Cadastro

| Caso | Comportamento |
| --- | --- |
| `nome_norm` já existe | recusa `DUPLICADO` |
| Nome a distância ≤ 2 de um existente | `SEMELHANTE` com candidatos, exige `confirmado` |
| `frota` já existe | recusa `DUPLICADO` |
| Frota a 1 dígito de uma existente | `SEMELHANTE` |
| Frota fora de 4–8 caracteres alfanuméricos | recusa `VALIDACAO` |
| Nome com menos de 3 caracteres | recusa `VALIDACAO` |

### Fusão

| Caso | Comportamento |
| --- | --- |
| Qualquer dos dois com viagem aberta | recusa `VALIDACAO` |
| Mesmo id nos dois lados | recusa `VALIDACAO` |

### Exclusão

| Caso | Comportamento |
| --- | --- |
| `motivo` vazio ou com menos de 3 caracteres | recusa `VALIDACAO` |
| Viagem já excluída | recusa `VALIDACAO` |

Sempre lógica: grava `excluida_em`, `excluida_por` e `excluida_motivo`, some das telas e dos relatórios, sai do índice único (liberando o veículo). O registro anterior completo vai para `auditoria` em JSON. **Não existe exclusão física em nenhum caminho da aplicação.**

O motivo é **obrigatório**: `motivo: String`, recusado vazio ou com menos de 3 caracteres com `VALIDACAO`. O Histórico o exibe na linha riscada. Exclusão sem motivo é a que ninguém consegue explicar três meses depois, e o campo só é preenchido se a assinatura obrigar.

---

## 7. Consultas dos relatórios

Duração em horas: `(julianday(dt_chegada) - julianday(dt_saida)) * 24`. Como as datas são ISO completas, não há adivinhação de virada de meia-noite.

Um detalhe que muda o número: viagens abertas **não entram** na soma de horas, mas **entram** na contagem de viagens. Um relatório que somasse horas de viagem sem chegada estaria inventando dado.

### Uso de veículo

```sql
SELECT v.frota, v.modelo,
       COUNT(*)                                      AS viagens,
       SUM(CASE WHEN s.dt_chegada IS NULL THEN 1 ELSE 0 END) AS abertas,
       ROUND(SUM(CASE WHEN s.dt_chegada IS NOT NULL
                 THEN (julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24
                 ELSE 0 END), 2)                     AS horas_totais,
       ROUND(AVG(CASE WHEN s.dt_chegada IS NOT NULL
                 THEN (julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24
                 END), 2)                            AS horas_media,
       SUM(CASE WHEN s.chegada_manual = 1 THEN 1 ELSE 0 END) AS chegadas_manuais,
       SUM(CASE WHEN s.hodometro_chegada IS NOT NULL
                 AND s.hodometro_saida IS NOT NULL
                THEN s.hodometro_chegada - s.hodometro_saida ELSE 0 END) AS km
FROM saidas s
JOIN veiculos v ON v.id = s.veiculo_id
WHERE s.excluida_em IS NULL
  AND s.dt_saida >= :de AND s.dt_saida < :ate
GROUP BY v.id
ORDER BY horas_totais DESC;
```

Cada linha expande no detalhamento de quem pegou o carro:

```sql
SELECT m.nome, s.dt_saida, s.dt_chegada, s.turno, s.destino, s.atividade,
       s.chegada_manual,
       CASE WHEN s.dt_chegada IS NULL THEN NULL
            ELSE ROUND((julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24, 2)
       END AS horas
FROM saidas s
JOIN motoristas m ON m.id = s.motorista_id
WHERE s.veiculo_id = :veiculo_id
  AND s.excluida_em IS NULL
  AND s.dt_saida >= :de AND s.dt_saida < :ate
ORDER BY s.dt_saida;
```

### Uso por motorista

```sql
SELECT m.nome, m.matricula,
       COUNT(*)                          AS viagens,
       COUNT(DISTINCT s.veiculo_id)      AS veiculos_distintos,
       GROUP_CONCAT(DISTINCT v.frota)    AS frotas,
       SUM(CASE WHEN s.dt_chegada IS NULL THEN 1 ELSE 0 END) AS abertas,
       ROUND(SUM(CASE WHEN s.dt_chegada IS NOT NULL
                 THEN (julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24
                 ELSE 0 END), 2)         AS horas_totais
FROM saidas s
JOIN motoristas m ON m.id = s.motorista_id
JOIN veiculos  v ON v.id = s.veiculo_id
WHERE s.excluida_em IS NULL
  AND s.dt_saida >= :de AND s.dt_saida < :ate
GROUP BY m.id
ORDER BY horas_totais DESC;
```

### Cabeçalho obrigatório

Todo relatório, em qualquer formato, abre com estes números. Sem eles o total de horas afirma algo que não sabe:

| Indicador | Cálculo |
| --- | --- |
| Período | `:de` a `:ate` |
| Viagens no período | `COUNT(*)` |
| Viagens sem chegada | contagem e % |
| Chegadas informadas manualmente | contagem e % |
| Viagens com hodômetro nas duas pontas | contagem e % |
| Emitido em / por | data-hora e usuário Windows |

### Notas de implementação

- `:ate` é **exclusivo**. Para "setembro inteiro", passar `2026-10-01`.
- `GROUP_CONCAT(DISTINCT ...)` não aceita separador customizado no SQLite; o padrão é vírgula. Se quiser outro, formatar no Rust.
- Formatos: HTML renderizado no próprio WebView, PDF pela impressão do WebView, CSV com BOM UTF-8 e separador `;` (como o sistema antigo já faz).
- Com a base nova, os volumes são pequenos (milhares de linhas). Nenhuma dessas consultas precisa de otimização além dos índices já definidos.

---

## 8. Critérios de aceite

Cada item é verificável por alguém do setor, sem ler código. A homologação não termina enquanto todos não passarem.

### Fase 0 — antes de qualquer funcionalidade

Protótipo descartável, rodando contra o compartilhamento real, em duas máquinas do setor:

- [ ] Gravar em loop no banco da rede, puxar o cabo no meio da escrita, reconectar, reabrir, rodar `integrity_check`. **20 vezes, variando o momento da interrupção, 20 aprovações.**
- [ ] Duas máquinas tentando o lock ao mesmo tempo: exatamente uma obtém.
- [ ] Matar o processo que detém o lock: a outra máquina consegue abrir sem intervenção.
- [ ] Medir o tempo de abertura do banco pela rede com o antivírus ativo.

Reprovou o primeiro item → **pare**: a arquitetura passa a ser cópia local com sincronismo, e a seção 4 precisa ser reescrita antes de continuar.

### Integridade e concorrência

- [ ] Abrir o app em duas máquinas: a segunda mostra a tela de bloqueio com o nome e a hora da primeira.
- [ ] Encerrar o processo pelo Gerenciador de Tarefas e reabrir na outra máquina: entra normalmente, sem intervenção manual.
- [ ] Desligar a máquina no botão durante o uso: o registro anterior está no banco ao reabrir.
- [ ] Abrir viagem para o veículo X, tentar abrir outra para o mesmo X: recusada, com o nome de quem está com ele.
- [ ] Idem para o mesmo motorista em outro veículo: recusada.
- [ ] Encerrar a primeira e reabrir: aceita.

### Dados

- [ ] Cadastrar "ANGELO JUNIOR" existindo "ANJELO JUNIOR": pergunta se é a mesma pessoa.
- [ ] Cadastrar frota `907218` existindo `907018`: pergunta.
- [ ] Fundir dois motoristas: as saídas de ambos aparecem sob o sobrevivente, e o total de viagens não muda.
- [ ] Excluir uma viagem: some das telas, o veículo fica livre, e ela aparece com o filtro de excluídas.
- [ ] Registrar chegada no dia seguinte (22:40 → 05:30): duração de 6h50, sem confirmação de duração longa.
- [ ] Registrar 08:00 → 07:00: pede confirmação de 23 h.
- [ ] Informar hodômetro de chegada menor que o de saída: recusado.

### Backup e relatórios

- [ ] Deixar o sistema aberto 4 h: backup criado com data-hora no nome.
- [ ] Fechar o sistema: backup e relatório gerados.
- [ ] **Restaurar um backup numa cópia e conferir que os dados estão lá.** Um backup nunca restaurado não é backup.
- [ ] Após 13 backups, o mais antigo do dia foi removido pela rotação.
- [ ] Simular queda de energia (desligar no botão) e reabrir: relatório do período pendente é gerado no login.
- [ ] Relatório de uso de veículo: totais conferem com contagem manual de uma semana.
- [ ] O cabeçalho mostra o percentual de viagens sem chegada.
- [ ] CSV abre no Excel com acentuação correta.

### Ambiente

- [ ] O `.exe` abre a partir do caminho UNC em todas as máquinas do setor.
- [ ] Nenhum aviso de SmartScreen, ou aviso já tratado com a TI.
- [ ] Abertura em menos de 5 segundos com o antivírus ativo.
- [ ] Desconectar o cabo de rede durante o uso: mensagem clara, sem perda do que foi digitado; reconectar volta ao normal.

### Piloto

- [ ] Uma semana registrando nos dois sistemas em paralelo, comparando os números ao fim.

A semana de piloto é o que dá confiança para desligar o HTML antigo sem sobressalto. Custa pouco e evita a descoberta desagradável no primeiro dia sozinho.

---

## Testes automatizados mínimos

O que é difícil de reproduzir à mão, e por isso precisa de teste:

| Teste | Verifica |
| --- | --- |
| `abrir_saida` duplicada para o mesmo veículo | `VEICULO_EM_USO`, não `BANCO` |
| `abrir_saida` duplicada para o mesmo motorista | `MOTORISTA_EM_USO` |
| Excluir viagem aberta e reabrir para o mesmo veículo | índice único libera com `excluida_em` |
| Duração 22:40 → 05:30 (dia seguinte) | 6,83 h, sem aviso |
| Duração > 14 h | devolve `DURACAO_LONGA` |
| `nome_norm` de "José  Carlos" vs "JOSE CARLOS" | colidem no `UNIQUE` |
| Levenshtein de "ANGELO JUNIOR" vs "ANJELO JUNIOR" | devolve `SEMELHANTE` |
| `fundir_motoristas` | contagem de saídas preservada |
| Rotação de backup com 40 arquivos sintéticos | mantém a política da seção 4 |
| Varredura de relatórios com 3 meses pendentes | gera os 3, do mais antigo ao mais novo |
| Varredura numa base nova, `ultimo_relatorio_periodo` vazio | não volta além do mês de `data_corte` |
| Migração de schema vazio → v1 | `user_version = 1`, tabelas e os dois índices únicos parciais criados |

Tela se testa olhando. Não perseguir cobertura ampla.

---

## Definição de pronto

- Compila sem warning.
- `cargo clippy` limpo.
- Teste cobrindo a regra alterada, quando ela estiver na tabela acima.
- Nenhum `unwrap()` ou `expect()` novo em caminho de execução.
- Mensagem de erro em português, com `codigo` estável.
