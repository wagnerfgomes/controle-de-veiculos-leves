//! Protótipo descartável da Fase 0.
//!
//! Responde às duas perguntas que decidem a arquitetura do projeto, e que hoje
//! ninguém sabe responder:
//!
//! 1. Um `.exe` não assinado roda numa máquina do setor?
//! 2. O SQLite sobrevive a uma queda de rede no meio de uma escrita, direto
//!    sobre a pasta compartilhada?
//!
//! Não usa Tauri de propósito: assim cross-compila do Arch para Windows com
//! `x86_64-pc-windows-gnu` mais `mingw-w64`, e o teste não exige montar
//! ambiente de desenvolvimento em máquina do setor.
//!
//! Este binário é jogado fora depois da Fase 0. Não o transforme em biblioteca
//! e não o importe do app.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rusqlite::Connection;

const USO: &str = "\
sonda — protótipo da Fase 0

USO:
  sonda martelo <caminho-do-banco>   grava uma transação por segundo, sem parar
  sonda checar  <caminho-do-banco>   integrity_check e confere a contagem
  sonda lock    <caminho-do-lock>    tenta adquirir o lock e segura até o Enter
  sonda tempo   <caminho-do-banco>   mede a abertura da conexão, 20 vezes

EXEMPLO (no Windows, contra a pasta de rede):
  sonda.exe martelo \\\\servidor\\logistica\\teste\\sonda.db
  ...puxe o cabo de rede no meio da escrita, reconecte, e então:
  sonda.exe checar  \\\\servidor\\logistica\\teste\\sonda.db

O teste da Fase 0 pede 20 repetições, variando o momento da interrupção, com
20 aprovações. Anote o momento e o resultado de cada uma.
";

fn main() {
    let argumentos: Vec<String> = std::env::args().collect();
    if argumentos.len() < 3 {
        print!("{USO}");
        std::process::exit(2);
    }

    let modo = argumentos[1].as_str();
    let caminho = PathBuf::from(&argumentos[2]);

    let resultado = match modo {
        "martelo" => martelo(&caminho),
        "checar" => checar(&caminho),
        "lock" => lock(&caminho),
        "tempo" => tempo(&caminho),
        outro => {
            eprintln!("modo desconhecido: {outro}\n");
            print!("{USO}");
            std::process::exit(2);
        }
    };

    if let Err(e) = resultado {
        eprintln!("FALHOU: {e}");
        std::process::exit(1);
    }
}

/// Os mesmos quatro PRAGMA do app. O teste não vale nada com outra
/// configuração: é justamente esta que está sendo posta à prova.
fn abrir(caminho: &Path) -> Result<Connection, String> {
    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta).map_err(|e| format!("ao criar {}: {e}", pasta.display()))?;
    }

    let conexao = Connection::open(caminho).map_err(|e| format!("ao abrir: {e}"))?;

    let modo: String = conexao
        .query_row("PRAGMA journal_mode = TRUNCATE", [], |l| l.get(0))
        .map_err(|e| format!("journal_mode: {e}"))?;
    if !modo.eq_ignore_ascii_case("truncate") {
        return Err(format!("o banco recusou journal_mode = TRUNCATE (ficou {modo})"));
    }

    conexao
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|e| format!("synchronous: {e}"))?;
    conexao
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("foreign_keys: {e}"))?;
    conexao
        .pragma_update(None, "busy_timeout", 5_000)
        .map_err(|e| format!("busy_timeout: {e}"))?;

    conexao
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS martelo (
                 n  INTEGER PRIMARY KEY,
                 em TEXT NOT NULL
             )",
        )
        .map_err(|e| format!("ao criar a tabela: {e}"))?;

    Ok(conexao)
}

/// Grava em loop, uma transação por segundo, imprimindo o número da última
/// confirmada. O número impresso é o contrato com o `checar`: tudo que apareceu
/// na tela **precisa** estar no banco depois da queda.
fn martelo(caminho: &Path) -> Result<(), String> {
    let conexao = abrir(caminho)?;
    println!("martelando {}", caminho.display());
    println!("puxe o cabo quando quiser. Ctrl+C para parar.\n");

    let mut n: i64 = conexao
        .query_row("SELECT IFNULL(MAX(n), 0) FROM martelo", [], |l| l.get(0))
        .map_err(|e| format!("ao ler o último número: {e}"))?;

    loop {
        n += 1;
        let agora = carimbo();

        let escrita = conexao.execute("INSERT INTO martelo (n, em) VALUES (?1, ?2)", (n, &agora));

        match escrita {
            Ok(_) => {
                // Só imprime depois do commit retornar. Com synchronous = FULL
                // isso significa que o flush já aconteceu.
                println!("confirmada {n}  {agora}");
                let _ = std::io::stdout().flush();
            }
            Err(e) => {
                println!("ERRO na {n}: {e}");
                println!("(esperado se a rede caiu. A última confirmada foi a {})", n - 1);
                let _ = std::io::stdout().flush();
                n -= 1;
            }
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}

/// Abre, roda `integrity_check` e confere se a contagem bate com o maior número
/// gravado. Buraco na sequência significa transação perdida, que é exatamente o
/// que reprova a Fase 0.
fn checar(caminho: &Path) -> Result<(), String> {
    let orfao = journal_orfao(caminho);
    if orfao {
        println!(
            "há um journal órfão ao lado do banco: o SQLite vai revertê-lo agora, sozinho.\n\
             NUNCA apague esse arquivo à mão."
        );
    }

    let conexao = abrir(caminho)?;

    let integridade: String = conexao
        .query_row("PRAGMA integrity_check", [], |l| l.get(0))
        .map_err(|e| format!("integrity_check: {e}"))?;

    let (total, maior): (i64, i64) = conexao
        .query_row("SELECT COUNT(*), IFNULL(MAX(n), 0) FROM martelo", [], |l| {
            Ok((l.get(0)?, l.get(1)?))
        })
        .map_err(|e| format!("ao contar: {e}"))?;

    println!("integrity_check ... {integridade}");
    println!("linhas .......... {total}");
    println!("maior número .... {maior}");
    if orfao {
        println!("journal órfão ... sim, revertido na abertura");
    }

    let integro = integridade == "ok";
    let sem_buraco = total == maior;

    if integro && sem_buraco {
        println!("\nAPROVADO: banco íntegro e sem buraco na sequência.");
        return Ok(());
    }

    if !integro {
        println!("\nREPROVADO: o banco está corrompido.");
    }
    if !sem_buraco {
        println!(
            "\nREPROVADO: faltam {} linha(s) na sequência. Alguma transação confirmada sumiu.",
            maior - total
        );
    }
    println!("Anote o momento da interrupção e repita. Uma reprovação já derruba a Fase 0.");
    std::process::exit(1);
}

fn journal_orfao(caminho: &Path) -> bool {
    let mut journal = caminho.as_os_str().to_os_string();
    journal.push("-journal");
    PathBuf::from(journal).exists()
}

/// Segura o lock até o Enter. Rodar nas duas máquinas ao mesmo tempo responde o
/// item da Fase 0: exatamente uma precisa obter.
fn lock(caminho: &Path) -> Result<(), String> {
    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta).map_err(|e| format!("ao criar {}: {e}", pasta.display()))?;
    }

    match adquirir(caminho) {
        Ok(arquivo) => {
            println!("LOCK OBTIDO em {}", caminho.display());
            println!("pid {} — segurando. Enter para liberar.", std::process::id());
            let mut linha = String::new();
            let _ = std::io::stdin().read_line(&mut linha);
            drop(arquivo);
            println!("liberado.");
            Ok(())
        }
        Err(e) => {
            println!("LOCK RECUSADO em {}", caminho.display());
            println!("motivo: {e}");
            println!("\nEste é o resultado esperado na segunda máquina.");
            std::process::exit(1);
        }
    }
}

#[cfg(windows)]
fn adquirir(caminho: &Path) -> Result<std::fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt;

    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0) // FILE_SHARE_NONE — é este o mecanismo sob teste.
        .open(caminho)
        .map_err(|e| e.to_string())
}

/// Fora do Windows o teste de lock **não conta**. O cliente SMB é outro e o
/// comportamento de lock é outro; serve só para exercitar o binário.
#[cfg(not(windows))]
fn adquirir(caminho: &Path) -> Result<std::fs::File, String> {
    eprintln!(
        "AVISO: fora do Windows este modo não testa nada de útil. \
         O lock de produção é o handle exclusivo do Windows."
    );
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(caminho)
        .map_err(|e| e.to_string())
}

/// Abertura mais os quatro PRAGMA, 20 vezes. O critério de aceite é abrir em
/// menos de 5 segundos com o antivírus ativo, e é aqui que se descobre se vai
/// ser preciso negociar exclusão de pasta com a TI.
fn tempo(caminho: &Path) -> Result<(), String> {
    let mut medidas = Vec::with_capacity(20);

    for i in 1..=20 {
        let inicio = Instant::now();
        let conexao = abrir(caminho)?;
        let decorrido = inicio.elapsed();
        drop(conexao);

        println!("{i:>2}: {:>8.1} ms", decorrido.as_secs_f64() * 1000.0);
        medidas.push(decorrido.as_secs_f64() * 1000.0);
    }

    medidas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let soma: f64 = medidas.iter().sum();
    let media = soma / medidas.len() as f64;
    let mediana = medidas[medidas.len() / 2];
    let pior = medidas.last().copied().unwrap_or(0.0);

    println!("\nmédia ... {media:.1} ms");
    println!("mediana . {mediana:.1} ms");
    println!("pior .... {pior:.1} ms");

    if pior > 5_000.0 {
        println!("\nATENÇÃO: a pior abertura passou de 5 s, que é o critério de aceite.");
        println!("Negocie com a TI a exclusão da pasta no antivírus.");
    }

    Ok(())
}

/// Sem chrono: a sonda tem uma dependência só, e é `rusqlite`. Quanto menos
/// coisa, menos chance de um `.exe` grande demais esbarrar na política da TI.
fn carimbo() -> String {
    let agora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{agora}")
}
