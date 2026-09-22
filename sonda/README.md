# Sonda — protótipo descartável da Fase 0

Responde às duas perguntas que decidem a arquitetura do projeto e que hoje ninguém sabe responder. É o melhor uso de um dia de trabalho no projeto inteiro, e não depende de nenhuma outra fase.

Este binário é jogado fora depois da Fase 0. Não o transforme em biblioteca, não o importe do app.

---

## Gerar o `.exe`

Falta uma única coisa na máquina de desenvolvimento, e ela precisa de `sudo`:

```bash
sudo pacman -S mingw-w64-gcc
rustup target add x86_64-pc-windows-gnu     # já instalado
cargo build --manifest-path sonda/Cargo.toml --release --target x86_64-pc-windows-gnu
```

O `.exe` sai em `sonda/target/x86_64-pc-windows-gnu/release/sonda.exe`. Copie para um pendrive.

Sem o `mingw-w64-gcc` a cross-compilação falha em `libsqlite3-sys`, que precisa de um compilador C para o alvo:

```
error occurred in cc-rs: failed to find tool "x86_64-w64-mingw32-gcc"
```

A versão de Linux compila normalmente e serve para exercitar o binário, **não** para o teste:

```bash
cargo build --manifest-path sonda/Cargo.toml --release
```

---

## Pergunta 1 — um `.exe` não assinado roda numa máquina do setor?

É a que ninguém fez ainda, e a mais barata de responder. Copie o `sonda.exe` para o **disco local** de uma máquina do setor e abra. Três desfechos:

| Desfecho | O que significa |
| --- | --- |
| Roda | Siga para a pergunta 2. O binário em si não é o problema |
| SmartScreen, com "Executar assim mesmo" | Roda, mas a versão final precisa de assinatura ou exceção da TI. Isso entra na conversa da apresentação |
| Bloqueado por política (AppLocker, WDAC) | **A descoberta mais importante do projeto**, e ela chegou antes de escrever uma linha do app. O modelo de entrega passa a depender da TI desde o primeiro dia |

---

## Pergunta 2 — o SQLite sobrevive a uma queda de rede?

Com o banco na pasta compartilhada. O `.exe` pode rodar do disco local ou do pendrive: quem precisa estar na rede é o **banco**, não o executável.

```
sonda.exe martelo \\servidor\logistica\teste\sonda.db
```

Ele grava uma transação por segundo e imprime o número da última confirmada. **Puxe o cabo de rede no meio da escrita.** Reconecte, e então:

```
sonda.exe checar \\servidor\logistica\teste\sonda.db
```

`APROVADO` significa banco íntegro e nenhum buraco na sequência: tudo que apareceu na tela está no banco.

**20 repetições, variando o momento da interrupção, 20 aprovações.** Planilha com momento e resultado de cada uma. Uma reprovação já derruba a Fase 0.

Se aparecer um `sonda.db-journal` ao lado do banco, **não apague**. É ele que permite a reversão, e o SQLite cuida disso sozinho na abertura seguinte.

### Reprovou?

Pare o desenvolvimento da Fase 4 em diante. A seção 4 do SPEC é reescrita para cópia local com sincronismo antes de qualquer outra coisa. As Fases 1, 2 e 3 seguem válidas, e o desenho de rede está confinado a três arquivos (`db/conexao.rs`, `lock.rs`, `sessao.rs`).

---

## Os outros dois modos

```
sonda.exe lock  \\servidor\logistica\teste\sonda.lock
```

Nas duas máquinas ao mesmo tempo: exatamente uma precisa obter. Matando o processo detentor pelo Gerenciador de Tarefas, a outra precisa conseguir sem intervenção nenhuma.

```
sonda.exe tempo \\servidor\logistica\teste\sonda.db
```

Mede abertura da conexão mais os quatro PRAGMA, 20 vezes, **com o antivírus ativo**. O critério de aceite é menos de 5 segundos. Passando disso, há tempo de negociar exclusão de pasta com a TI antes da entrega.

---

## O teste fraco, se a pergunta 1 impedir rodar qualquer `.exe`

Rodar o `martelo` do próprio notebook Linux contra a mesma pasta, por CIFS. **Serve só como teste negativo:** falhando ali, falha em Windows com folga; passando, não prova nada, porque o cliente SMB do Windows não é o do Linux e o comportamento de cache e de lock é outro.

Não marque o critério da seção 8 do SPEC com esse resultado.
