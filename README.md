# Controle de Veículos Leves

Aplicação desktop que controla saída e retorno de veículos leves do setor de Logística da usina. Substitui um HTML monolítico de 7.454 linhas onde os dados viviam num array JavaScript dentro do próprio arquivo.

**Tauri 2, React, SQLite.** Roda de um `.exe` numa pasta de rede Windows, com o banco ao lado dele e um usuário por vez. Sem servidor, sem instalador, sem nada gravado na máquina do usuário além do log.

```bash
npm install
npm run build:demo    # gera a demonstração (Linux ou Windows)
npm run build:exe     # gera o executável de entrega (só no Windows)
npm run verificar     # tipos, clippy e a suíte de testes
```

O executável de entrega também sai pronto pelo GitHub Actions, sem precisar de máquina Windows. Veja [Parte 1](#parte-1-para-quem-desenvolve).

---

Três partes, três públicos:

| Parte | Para quem | O que cobre |
| --- | --- | --- |
| [1](#parte-1-para-quem-desenvolve) | Quem desenvolve | Setup, testes, build da demo, sonda da Fase 0a |
| [2](#parte-2-instalar-na-pasta-de-rede-e-cuidar-dela) | Quem instala e responde pelo sistema | Onde cada arquivo fica, o que precisa em cada máquina, **backup sem estragar o banco** |
| [3](#parte-3-para-quem-vai-usar) | Quem opera no dia a dia | Manual de uso, sem nada de código. Pode imprimir e entregar |

O contrato técnico completo está em [`docs/SPEC.md`](docs/SPEC.md), e a ordem de execução em [`docs/PLANO.md`](docs/PLANO.md). Este arquivo não os substitui: ele ensina a rodar o que já existe.

---

# Parte 1: para quem desenvolve

## O que é

Aplicação desktop (Tauri 2 com React e SQLite) que controla saída e retorno de veículos leves do setor de Logística. O `.exe` mora numa pasta de rede Windows, o banco fica ao lado dele, e **um usuário opera por vez**, garantido por um lock de handle exclusivo.

Cinco restrições definem o projeto e não devem ser contornadas. Elas estão no [`CLAUDE.md`](CLAUDE.md), e a mais importante para entender qualquer decisão do código é esta: não existe API, servidor nem cópia local do banco. A durabilidade vem da configuração da conexão, e a segurança de manter o banco em rede vem do lock.

## O que você precisa instalado

| Ferramenta | Versão testada | Para quê |
| --- | --- | --- |
| Rust | 1.93 | backend |
| Node | 22 | front e CLI do Tauri |
| `webkit2gtk-4.1` | 2.52 | janela no Linux |
| `mingw-w64-gcc` | qualquer | só para gerar o `.exe` da sonda |

No Arch:

```bash
sudo pacman -S rust nodejs npm webkit2gtk-4.1 base-devel
```

O bundle de produção para o setor é gerado **no Windows**, porque o alvo é WebView2. Build no Linux serve para checagem de compilação e para a demonstração, nunca para entrega.

## Primeira vez

```bash
git clone git@github.com:wagnerfgomes/controle-de-veiculos-leves.git
cd controle-de-veiculos-leves
npm install
```

O `npm install` baixa cerca de 110 MB. Se a rede estiver ruim, use retries mais longos:

```bash
npm install --fetch-retries=15 --fetch-timeout=900000
```

Se o `npx tauri` reclamar de `Cannot find module './cli.linux-x64-gnu.node'`, o binário nativo do CLI não foi instalado (acontece quando a rede derruba as dependências opcionais):

```bash
npm install --no-save @tauri-apps/cli-linux-x64-gnu
```

## Rodar em desenvolvimento

```bash
npm run tauri dev
```

Abre a janela com recarga quente do front e recompilação do Rust. Sem `config.toml` ao lado do executável, o app cai num padrão de desenvolvimento: modo produção apontando para a pasta local `dados-dev`, e o banco persiste entre execuções. O código não sabe a diferença entre essa pasta e um caminho UNC, e é justamente isso que permite desenvolver sem rede.

O aviso `LOCK EM MODO DE DESENVOLVIMENTO` no log e a etiqueta laranja na barra de status são esperados fora do Windows. Eles existem para você nunca confundir o stub de `flock` com o lock de verdade.

## Verificar antes de dar qualquer coisa por pronta

```bash
npx tsc --noEmit                                                        # tipos do front
npm run build                                                           # bundle do front
cargo build   --manifest-path src-tauri/Cargo.toml                      # sem warning
cargo clippy  --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test    --manifest-path src-tauri/Cargo.toml                      # 87 testes
```

A definição de pronto do projeto exige os quatro limpos, mais teste cobrindo a regra alterada quando ela estiver na tabela de testes mínimos do SPEC.

Para rodar um teste só:

```bash
cargo test --manifest-path src-tauri/Cargo.toml nome_do_teste -- --exact --nocapture
```

## Gerar o build da demonstração

É o binário que vai para a apresentação. **Não use `cargo build --release`**, e essa é a armadilha mais cara do projeto:

```bash
npx tauri build --no-bundle --features demo
./src-tauri/target/release/controle-veiculos
```

Quem decide entre o dev server e o bundle embutido é o CLI do Tauri, pela cfg `dev`. Um `cargo build --release` sai marcado como dev, e o binário continua apontando para `http://localhost:1420`. Ele abre, a janela fica em branco, nenhum comando é chamado, e nada no log diz o motivo. Já custou um diagnóstico inteiro.

A feature `demo` força `modo = "demonstracao"` em tempo de compilação. Não há como apontar a apresentação para dado real por engano, e é o único release de Linux que o projeto permite: um release de Linux sem `--features demo` nem compila, por `compile_error!`.

O que a demo faz a cada abertura:

- recria a base do zero em `dados-demo`, ao lado do executável;
- gera 132 viagens fictícias em dois meses, 4 delas abertas, uma com mais de 24 h para a linha vermelha aparecer;
- cadastra `ANGELO JUNIOR` e `ANJELO JUNIOR` de propósito, para o alerta de nome parecido ter o que alertar;
- mantém o lock ativo, backup e relatórios automáticos desligados, e emissão manual de relatório ligada.

O gerador é determinístico: a mesma apresentação duas vezes mostra os mesmos números, o que ajuda a ensaiar.

### Roteiro sugerido, na ordem que convence

1. Painel com os carros na rua e o tempo de cada um.
2. Abrir uma viagem em vinte segundos.
3. Encerrar com um clique.
4. Tentar abrir o mesmo veículo de novo e mostrar a recusa, com o nome de quem está com ele.
5. Cadastrar um nome parecido e mostrar o alerta de duplicata.
6. Emitir o relatório do mês.
7. Abrir duas instâncias e mostrar a tela de bloqueio.

### Três coisas que a demo no Linux não prova

Não prometa nenhuma delas na apresentação:

- **O lock demonstrado é o stub de `flock`**, não o `share_mode(0)` do Windows. A tela de bloqueio aparece igual e o comportamento é honesto, mas o mecanismo que garante usuário único **em rede** não foi exercitado. Ele será testado na Fase 0b.
- **A renderização é WebKitGTK, não WebView2.** Fonte, espaçamento e detalhes de CSS mudam entre os dois. O que ficou perfeito na projeção pode desalinhar no Windows.
- **Nada sobre a política de executáveis do setor foi respondido.** Isso é assunto da sonda, abaixo.

## A sonda da Fase 0a

Um CLI descartável, sem Tauri, que responde as duas perguntas que decidem a arquitetura do projeto. É o melhor uso de um dia de trabalho aqui, e não depende de nenhuma outra fase. O procedimento completo está em [`sonda/README.md`](sonda/README.md); o resumo:

```bash
sudo pacman -S mingw-w64-gcc
rustup target add x86_64-pc-windows-gnu
cargo build --manifest-path sonda/Cargo.toml --release --target x86_64-pc-windows-gnu
```

O `.exe` sai em `sonda/target/x86_64-pc-windows-gnu/release/sonda.exe`. Leve num pendrive.

**Pergunta 1, a que ninguém fez ainda:** um `.exe` não assinado roda numa máquina do setor? Copie para o disco local e abra. Se for bloqueado por política (AppLocker, WDAC), essa é a descoberta mais importante do projeto, e ela chegou antes de escrever uma linha do app.

**Pergunta 2, o teste de estresse:** com o banco na pasta compartilhada,

```
sonda.exe martelo \\servidor\logistica\teste\sonda.db
```

grava uma transação por segundo e imprime o número da última confirmada. Puxe o cabo de rede no meio da escrita, reconecte, e rode:

```
sonda.exe checar \\servidor\logistica\teste\sonda.db
```

São necessárias **20 repetições, variando o momento da interrupção, com 20 aprovações**. Uma reprovação já derruba a arquitetura atual e obriga a reescrever a seção 4 do SPEC para cópia local com sincronismo.

Se aparecer um arquivo `sonda.db-journal` ao lado do banco, **não apague**. É ele que permite a reversão, e o SQLite cuida disso sozinho na próxima abertura. A mesma regra vale para o `controle.db-journal` em produção.

## Mapa do repositório

```
CLAUDE.md               as cinco restrições, as convenções e as armadilhas
README.md               este arquivo
config.toml.exemplo     modelo do config que fica ao lado do .exe
docs/SPEC.md            contrato de implementação completo
docs/PLANO.md           ordem de execução em fases
legado/                 HTML original. Referência de domínio, não é código vivo
sonda/                  CLI descartável da Fase 0a
src/                    React e TypeScript
  api/                  um wrapper de invoke() por comando Rust
  tipos/                espelho TS das structs serde
  telas/                uma tela por fluxo
  componentes/          reutilizáveis, sem regra de negócio
  estilo/               tokens herdados do HTML legado
src-tauri/
  migracoes/            SQL versionado
  src/
    main.rs             bootstrap, registro dos comandos, heartbeat
    comandos/           um arquivo por agregado
    dominio/            regra de negócio pura, testável sem banco
    db/                 conexão, PRAGMA, migrações, backup, relatórios
    lock.rs             handle exclusivo e heartbeat
    sessao.rs           sequência de abertura da seção 4
    demo.rs             gerador da base fictícia
```

Regra que organiza tudo: **comando orquestra transação, auditoria e tradução de erro, e nada mais**. Regra de negócio nova entra em `dominio/`. Cada comando é uma casca fina sobre uma função `*_com(&mut Connection, ...)`, e é isso que permite testar as regras sem subir o Tauri inteiro.

## Armadilhas que já custaram caro

**O SQLite não cita o nome do índice ao violar um índice único parcial.** A mensagem é `UNIQUE constraint failed: saidas.veiculo_id`, indistinguível de um `UNIQUE` de coluna. Casar por `ux_veiculo_em_uso` sozinho é um ramo que nunca dispara, e aí `VEICULO_EM_USO` vaza como `BANCO`, que é exatamente o que o operador não pode ver. Teste que fabrica a mensagem suposta passa; só um teste que provoca a violação contra banco real pega isso.

**Em modo demonstração, o lock vem antes de recriar a base.** A ordem inversa fazia a segunda instância apagar o banco da primeira antes de descobrir que não podia abrir. Como o roteiro da apresentação pede justamente abrir duas instâncias, isso destruiria a base no meio da demo.

**"Aberta" significa `dt_chegada IS NULL AND excluida_em IS NULL`**, sempre as duas condições. Filtro incompleto ressuscita registro excluído ou esconde veículo que está na rua.

**Os quatro PRAGMA rodam em toda conexão aberta.** Não são padrão do SQLite e não são herdados entre conexões. Há um teste que os lê de volta justamente para isso não se perder.

## O que ainda falta

| Fase | O que é | Por que não está pronta |
| --- | --- | --- |
| 0a | Sonda no setor | Exige máquina Windows do setor e o `mingw-w64-gcc` instalado |
| 8 | A apresentação em si | O build está pronto; falta ensaiar e apresentar |
| 0b | Lock em duas máquinas, no UNC definitivo | Depende da autorização que a apresentação destrava |
| 9 | Endurecimento de rede | Depende do veredito da Fase 0 |
| 10 | Build Windows, homologação e piloto | Depende de todas as anteriores |

O que a Fase 9 traz, e por que foi deixada de fora de propósito: modo degradado, `reconectar()`, `integrity_check` com desvio para `backups\corrompidos\`, backup automático a cada 4 h, rotação e varredura de relatórios pendentes. O heartbeat já bate e já conta as falhas, mas **não** derruba para modo degradado: ligar isso sem `reconectar()` deixaria o operador preso numa tela que recusa tudo e não oferece caminho de volta.

---

# Parte 2: instalar na pasta de rede, e cuidar dela

Esta parte é para quem coloca o sistema no ar e responde por ele. Ela responde três perguntas: onde cada arquivo fica, o que é preciso ter em cada máquina, e como fazer backup sem estragar o banco.

## A pasta é a instalação inteira

Não existe instalador, serviço, banco de dados em servidor nem registro no Windows. O sistema é uma pasta, e é isso que permite abrir de qualquer estação sem procedimento nenhum.

```
\\servidor\logistica\veiculos-leves\          a pasta, que é tudo
├── ControleVeiculos.exe                      o programa
├── config.toml                               diz onde ficam os dados e o modo
├── dados\
│   ├── controle.db                           O BANCO. É o arquivo que importa
│   ├── controle.db-journal                   recuperação de queda. NUNCA apague
│   ├── controle.lock                         o "tem alguém usando"
│   └── controle.lock.info                    quem é, em que máquina, desde quando
├── backups\
│   └── controle_2026-09-22_1430.db           cópias consistentes, uma por geração
└── relatorios\
    └── 2026-09\
        └── uso-veiculo_2026-09-01_a_2026-10-01.csv
```

As subpastas nascem sozinhas na primeira abertura. Você só precisa criar a pasta principal e colocar dois arquivos nela: o executável e o `config.toml`.

O `config.toml` fica **ao lado do executável**, nunca compilado dentro dele. Trocar de servidor é editar um arquivo de texto, não recompilar:

```toml
caminho_dados = "\\\\servidor\\logistica\\veiculos-leves"
modo          = "producao"
```

Repare nas barras dobradas. É exigência do formato TOML, e o `config.toml.exemplo` do repositório já vem assim.

### O que é cada arquivo, em uma linha

| Arquivo | Para que serve | Pode apagar? |
| --- | --- | --- |
| `controle.db` | O banco. Todas as viagens, veículos e condutores | **Nunca** |
| `controle.db-journal` | Permite o SQLite reverter uma escrita interrompida | **Nunca.** Ver abaixo |
| `controle.lock` | Arquivo travado enquanto alguém usa | Não, mas o sistema o recria |
| `controle.lock.info` | Texto legível com quem está usando | Sim, mas some sozinho ao fechar |
| `backups\*.db` | Cópias do banco, consistentes | Sim, a rotação faz isso |
| `relatorios\` | Arquivos emitidos | Sim |

**Sobre o `controle.db-journal`:** ele quase sempre aparece com tamanho zero, e isso é normal. Se o sistema cair no meio de uma gravação, ele fica com conteúdo, e é ele que o SQLite lê na próxima abertura para desfazer a escrita pela metade. Apagá-lo à mão é o que transforma uma queda recuperável em perda de dados. Não apague, e não oriente ninguém a apagar.

## O que precisa estar em cada máquina do setor

Quase nada, mas esse "quase" importa.

| O que | Onde | Observação |
| --- | --- | --- |
| **WebView2 Runtime** | na máquina | **A única dependência externa.** Sem ele o programa abre e não desenha tela |
| `app.log` | `%LOCALAPPDATA%\ControleVeiculos\app.log` | Único arquivo que o sistema escreve fora da pasta de rede. Só log, nenhum dado |

O WebView2 vem por padrão no Windows 11 e na maioria dos Windows 10 atualizados, mas **não é garantido**, e as máquinas do setor podem não ter internet para buscá-lo sozinhas. Vale confirmar isso na mesma ida em que você for rodar a sonda da Fase 0a, junto com a pergunta sobre política de executáveis.

Para conferir numa máquina, no PowerShell:

```powershell
Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue |
  Select-Object pv
```

Se devolver um número de versão, o runtime está presente. Se não devolver nada, a TI precisa instalá-lo, e isso entra na mesma conversa da liberação do executável.

O `app.log` é a primeira coisa a olhar quando alguém disser que o sistema não abriu naquela máquina. Ele registra a abertura, o caminho tentado, a recusa de lock com o nome de quem está usando, e qualquer falha de rede.

## Mover, copiar ou renomear a pasta

A pasta inteira é portátil, com duas ressalvas.

**Move sem problema:** copie a pasta para outro caminho, ajuste `caminho_dados` no `config.toml`, e pronto. Os backups e relatórios vão junto. Nada no banco guarda o caminho de onde ele estava.

**Faça isso com o sistema fechado.** Copiar a pasta enquanto alguém está usando cai no mesmo problema do backup, explicado logo abaixo.

**Não rode duas cópias da pasta apontando para o mesmo `caminho_dados`.** O lock protege contra duas instâncias no mesmo banco, então a segunda cai na tela de bloqueio, mas você acaba com dois executáveis de versões diferentes gravando no mesmo lugar, e a primeira migração de schema que uma delas aplicar deixa a outra sem entender o banco.

## Backup do banco por fora do sistema

Aqui está o ponto que merece atenção, e ele contraria o instinto.

**Não copie o `controle.db` com `copy`, `xcopy` ou `robocopy` enquanto o sistema estiver aberto.** Um banco SQLite em uso está sendo escrito em páginas, e uma cópia feita no meio de uma gravação pode capturar páginas de dois estados diferentes. O arquivo resultante **parece** um backup: tem o tamanho certo, abre sem reclamar, e só revela o problema no dia em que você precisa restaurá-lo. É o pior tipo de falha, porque ela fica invisível até a hora em que doer.

Existem três caminhos seguros. Em ordem de preferência para este caso:

### 1. Copie a pasta `backups\`, não o banco vivo

O sistema gera as cópias com `VACUUM INTO`, que produz um arquivo consistente por construção. Esses arquivos já estão fechados e prontos quando aparecem, então copiá-los é seguro a qualquer momento, com qualquer ferramenta.

```powershell
robocopy "\\servidor\logistica\veiculos-leves\backups" "D:\backup-veiculos" /MIR /R:3 /W:10
```

Agendado a cada 3 horas no Agendador de Tarefas, isso resolve sem risco nenhum e sem programa extra.

**A ressalva:** hoje o sistema gera backup **ao fechar** e pelo botão **Backup agora** na barra de status. O backup automático a cada 4 horas está previsto, mas é da Fase 9 e ainda não foi implementado. Na prática, se a estação ficar dias sem fechar o programa, não aparece backup novo para o seu script copiar.

Se você quer os 3 em 3 horas de verdade, o caminho limpo é ligar o backup automático do próprio sistema. É um item pequeno e já contratado no SPEC. Me peça e eu implemento.

### 2. Use o `sqlite3.exe` com o comando de backup online

Funciona mesmo com o sistema aberto, porque usa a API de backup do próprio SQLite em vez de copiar bytes:

```powershell
sqlite3.exe "\\servidor\logistica\veiculos-leves\dados\controle.db" ".backup 'D:\backup-veiculos\controle_$(Get-Date -f yyyy-MM-dd_HHmm).db'"
```

O `sqlite3.exe` é um único arquivo de cerca de 1 MB, não precisa de instalação. **Mas ele é mais um executável**, e passa pela mesma política de TI que o sistema. Se o setor bloqueia executável não assinado, este também vai barrar.

### 3. Copiar o banco com o sistema fechado

Se o setor usa o sistema em horário definido, uma cópia simples de madrugada é perfeitamente segura. Nesse caso copie **os dois arquivos juntos**, `controle.db` e `controle.db-journal`, porque o journal faz parte do estado do banco.

### Teste a restauração, pelo menos uma vez

Um backup nunca restaurado não é backup, é um arquivo. Pegue uma cópia, coloque numa pasta separada com um `config.toml` apontando para lá, abra o sistema e confira se as viagens estão todas lá. Faça isso uma vez agora, no começo, e não descubra em março que a rotina vinha gravando lixo desde outubro.

## Quando alguma coisa der errado

| Sintoma | Onde olhar | O que costuma ser |
| --- | --- | --- |
| Não abre em uma máquina só | `%LOCALAPPDATA%\ControleVeiculos\app.log` daquela máquina | WebView2 ausente, ou política de executável |
| "Sistema em uso" e não tem ninguém | `dados\controle.lock.info` | Estação que travou. Depois de 10 min sem sinal, o sistema oferece assumir |
| Apareceu `controle.db-journal` com tamanho grande | nada a fazer | Queda no meio de uma escrita. O sistema reverte sozinho na próxima abertura |
| "Conexão com o servidor perdida" | rede e o compartilhamento | O servidor de arquivos saiu do ar. Reconectar resolve |

---

# Parte 3: para quem vai usar

Este sistema registra qual veículo saiu, com quem, para onde e a que horas voltou. Ele substitui a planilha, e a diferença principal é que ele **não deixa** dois registros conflitantes existirem.

## A tela inicial

O Painel responde a pergunta que o setor faz o tempo todo: quais carros estão na rua e há quanto tempo.

- **Em cima**, os números do período: total de viagens, quantos carros estão na rua agora, quantos condutores e quantas frotas rodaram, a duração média e a divisão por turno.
- **No meio**, as viagens abertas, da mais antiga para a mais nova, com o tempo decorrido de cada uma. Passando de 24 horas, **a linha fica vermelha**. Cada linha tem o botão **Encerrar**.
- **Embaixo**, os veículos disponíveis. Os que estão em viagem aparecem apagados, com o nome de quem está com eles, para você saber a quem recorrer.
- **No rodapé**, seu usuário, a máquina, o estado da conexão com o servidor e a data do último backup.

Quando a duração média aparece como `—`, não é erro: significa que nenhuma viagem fechou ainda no período. Um zero ali afirmaria que os carros rodaram sem gastar tempo.

## Registrar uma saída

Botão **Nova saída**. Os campos estão na ordem em que a informação costuma chegar pelo rádio:

1. **Veículo.** Digite a frota ou o modelo. Os que estão em viagem aparecem apagados e não podem ser escolhidos.
2. **Condutor.** Digite o nome ou a matrícula.
3. **Turno.** Já vem marcado pelo horário, mas **continue conferindo**: a regra real depende da escala, não do relógio.
4. **Saída.** Já vem preenchida com o horário atual, e é editável.
5. **Destino.** Opcional. Ele sugere os destinos que já foram usados.
6. **Atividade.** Obrigatório. O que o veículo vai fazer.
7. **Hodômetro.** Opcional, já preenchido com o último valor conhecido daquele carro.
8. **Observação.** Opcional.

Se o veículo ou o condutor ainda não existir, digite o nome e o sistema oferece **Cadastrar**, sem sair da tela.

## Encerrar uma viagem

No Painel, botão **Encerrar** na linha da viagem. O caminho normal é de um clique: **Encerrar agora** usa o horário atual.

Se a viagem terminou antes e você está registrando depois, clique em **informar outro horário** e digite a data e a hora reais. O sistema marca essas chegadas e o relatório mostra quantas foram informadas à mão, o que é a forma de saber se o registro está sendo feito na hora ou no fim do turno.

O hodômetro de chegada é opcional.

## Quando o sistema pergunta alguma coisa

Ele só pergunta quando o número parece estranho, e sempre dá para confirmar:

| O que aparece | Por quê | O que fazer |
| --- | --- | --- |
| "A viagem durou 16h20. Confirma?" | Passou de 14 horas | Se o turno foi longo mesmo, confirme. Se digitou errado, corrija |
| "A saída está sendo registrada com 12 dias de atraso" | Data de saída muito antiga | Confirme se é registro atrasado mesmo |
| "Hodômetro registrado 45.200, informado 44.900" | Menor que o último conhecido | Confira o painel do carro |
| "A viagem teria 1.300 km. Confirma?" | Diferença grande de hodômetro | Confirme se a viagem foi longa mesmo |

E estas ele **recusa**, porque o registro ficaria errado:

- chegada antes da saída;
- saída ou chegada no futuro;
- hodômetro de chegada menor que o de saída;
- viagem já encerrada;
- veículo ou condutor já em viagem.

Registrar chegada no dia seguinte é normal e **não** gera pergunta: sair às 22:40 e voltar às 05:30 dá 6h50, e o sistema entende isso sozinho.

## Cadastrar veículo e condutor

Aba **Gestão**, abas **Veículos** e **Condutores**.

A frota precisa ter de 4 a 8 caracteres, só letras e números. O nome precisa ter pelo menos 3 letras.

## Quando aparece "Já existe ANJELO JUNIOR. É a mesma pessoa?"

Essa pergunta é o ponto mais importante do sistema inteiro.

O sistema antigo acumulou **182 nomes de condutor para cerca de 70 pessoas reais**, porque o campo era texto livre e cada um digitava de um jeito. Com isso, nenhum relatório por condutor significava nada.

Quando o nome digitado se parece com um que já existe, o sistema pergunta **antes de gravar**:

- **É a mesma pessoa?** Escolha o cadastro que já existe. Nada novo é criado.
- **É outra pessoa mesmo?** Confirme, e o cadastro novo é criado.

Na dúvida, escolha o existente. É muito mais fácil separar depois do que juntar.

## Juntar dois cadastros que eram a mesma pessoa

Na aba **Gestão**, quando existem cadastros parecidos, aparece um aviso **Possíveis duplicatas** no topo, com o botão **Fundir** em cada par.

Fundir move todas as viagens do cadastro duplicado para o que você escolheu manter, e apaga o duplicado. O total de viagens não muda, nada se perde, e fica registrado quem fez e quando.

Só não é possível fundir se algum dos dois estiver em viagem no momento. Encerre a viagem primeiro.

## Histórico, correção e exclusão

Aba **Histórico**. Filtre por período, turno, veículo ou condutor. O campo **Até é exclusivo**: para ver setembro inteiro, coloque de `2026-09-01` até `2026-10-01`.

Para excluir uma viagem lançada errado, use **Excluir** na linha. **O motivo é obrigatório.** Ele fica guardado e aparece na linha riscada quando você liga **Mostrar excluídas**.

A viagem excluída some das telas e dos relatórios, e o veículo volta a ficar disponível na hora. Nada é apagado de verdade: exclusão sem motivo é a que ninguém consegue explicar três meses depois, e é por isso que o campo é obrigatório.

## Relatórios

Aba **Relatórios**. Dois tipos, **Uso de veículo** e **Uso por condutor**. Escolha o período, clique em **Ver prévia** para conferir na tela, e depois **Gerar CSV** (abre no Excel) ou **Gerar HTML** (abre no navegador e imprime em PDF).

Todo relatório começa com os números que dizem **o quanto ele é confiável**:

- quantas viagens no período;
- quantas ficaram **sem chegada registrada**, e a porcentagem;
- quantas chegadas foram **informadas à mão**, e a porcentagem;
- quantas têm **hodômetro nas duas pontas**, e a porcentagem.

Olhe sempre esses números antes de usar o total de horas. Se 30% das viagens estão sem chegada, o total de horas está contando menos do que aconteceu de verdade.

Viagem aberta **conta como viagem** mas **não soma horas**, porque o sistema não inventa o horário de chegada que ninguém informou.

## "Sistema em uso"

Se aparecer uma tela dizendo **Sistema em uso**, com o nome de uma pessoa, uma máquina e um horário, significa que alguém já está com o sistema aberto. Ele funciona com **uma pessoa por vez**, e é isso que mantém os dados seguros na pasta de rede.

O certo é **procurar essa pessoa**. Clique em **Tentar novamente** quando ela fechar.

Se o computador da outra pessoa travou ou foi desligado no botão, depois de 10 minutos sem sinal aparece a opção de assumir a sessão, e ela exige digitar `CONFIRMAR`. **Use só se tiver certeza de que aquele sistema está realmente fechado**, porque assumir enquanto o outro computador ainda está gravando pode corromper os dados.

## Onde ficam os dados

Tudo fica na pasta de rede, junto com o programa. **Nada é gravado no seu computador**, e isso é de propósito: você pode abrir o sistema de qualquer estação do setor e vai encontrar exatamente os mesmos dados, sem instalar nada e sem pedir nada para ninguém.

Não existe "salvar". Cada saída registrada e cada chegada encerrada já está gravada no momento em que você confirma.

Se você abrir a pasta do sistema, duas coisas merecem atenção:

- **Não apague nada da pasta `dados`.** Em especial um arquivo chamado `controle.db-journal`, que costuma aparecer com tamanho zero. Ele existe justamente para o sistema se recuperar sozinho de uma queda de energia ou de rede, e apagá-lo é o que faria os registros se perderem de verdade.
- **Os relatórios que você emite** ficam em `relatorios`, organizados por mês. Pode copiar e enviar à vontade.

A única coisa que fica no seu computador é um arquivo de registro de funcionamento, em `%LOCALAPPDATA%\ControleVeiculos\app.log`. Ele não contém dado de viagem nenhum, e serve para o suporte entender o que aconteceu quando algo dá errado. Se precisar pedir ajuda, é esse arquivo que vale enviar junto.

## Quando alguma coisa dá errado

| O que aparece | O que significa | O que fazer |
| --- | --- | --- |
| "Conexão com o servidor perdida" | A rede caiu | Confira o cabo e a rede, e reconecte. O último registro pode não ter sido salvo, confira no Histórico |
| "Este veículo já está em viagem" | O carro não voltou | Encerre a viagem anterior primeiro. A mensagem diz quem está com ele |
| "Este condutor já está em viagem" | A pessoa está com outro carro | Encerre a viagem anterior |
| Uma faixa laranja **MODO DEMONSTRAÇÃO** | Você está na versão de apresentação | **Não registre nada de verdade aqui.** Tudo é apagado ao reabrir |

Se aparecer um arquivo chamado `controle.db-journal` na pasta do sistema, **não apague**. Ele existe justamente para o sistema se recuperar sozinho de uma queda, e apagá-lo é o que faria os dados se perderem de verdade.
