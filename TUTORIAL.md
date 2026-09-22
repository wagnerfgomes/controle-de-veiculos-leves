# Tutorial

Duas partes, dois públicos. A **Parte 1** é para quem desenvolve e faz o build. A **Parte 2** é o manual de quem vai usar o sistema no dia a dia, e pode ser impressa e entregue sem nada de código junto.

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
TUTORIAL.md             este arquivo
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

# Parte 2: para quem vai usar

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

## Quando alguma coisa dá errado

| O que aparece | O que significa | O que fazer |
| --- | --- | --- |
| "Conexão com o servidor perdida" | A rede caiu | Confira o cabo e a rede, e reconecte. O último registro pode não ter sido salvo, confira no Histórico |
| "Este veículo já está em viagem" | O carro não voltou | Encerre a viagem anterior primeiro. A mensagem diz quem está com ele |
| "Este condutor já está em viagem" | A pessoa está com outro carro | Encerre a viagem anterior |
| Uma faixa laranja **MODO DEMONSTRAÇÃO** | Você está na versão de apresentação | **Não registre nada de verdade aqui.** Tudo é apagado ao reabrir |

Se aparecer um arquivo chamado `controle.db-journal` na pasta do sistema, **não apague**. Ele existe justamente para o sistema se recuperar sozinho de uma queda, e apagá-lo é o que faria os dados se perderem de verdade.
