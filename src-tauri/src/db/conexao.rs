//! Abertura da conexão e os quatro PRAGMA que tornam seguro manter o banco
//! direto na pasta de rede.
//!
//! Nenhum dos quatro é padrão do SQLite e nenhum é herdado entre conexões: toda
//! conexão aberta em qualquer ponto do código passa por aqui.

use std::path::Path;

use rusqlite::Connection;

use crate::erro::ErroApp;

/// - `journal_mode = TRUNCATE`: WAL **não** funciona sobre SMB, depende de
///   memória compartilhada entre processos que o compartilhamento não oferece.
/// - `synchronous = FULL`: força o flush antes do commit retornar, anulando o
///   cache de escrita do cliente SMB. É daqui que vem a durabilidade, e é o que
///   dispensa qualquer etapa posterior de sincronismo.
/// - `foreign_keys = ON`: desligado por padrão no SQLite, por compatibilidade.
/// - `busy_timeout = 5000`: cobre a latência de rede em vez de devolver
///   `SQLITE_BUSY` na cara do operador.
pub fn aplicar_pragmas(conexao: &Connection) -> Result<(), ErroApp> {
    // journal_mode devolve uma linha com o modo efetivo, então não passa por
    // pragma_update. Ignorar o retorno esconderia uma troca recusada.
    let modo: String = conexao.query_row("PRAGMA journal_mode = TRUNCATE", [], |l| l.get(0))?;
    if !modo.eq_ignore_ascii_case("truncate") {
        return Err(ErroApp::banco(
            "O banco recusou journal_mode = TRUNCATE, que é o único modo seguro sobre a rede.",
        )
        .com_detalhe(format!("journal_mode efetivo: {modo}")));
    }

    conexao.pragma_update(None, "synchronous", "FULL")?;
    conexao.pragma_update(None, "foreign_keys", "ON")?;
    conexao.pragma_update(None, "busy_timeout", 5_000)?;

    Ok(())
}

pub fn abrir(caminho: &Path) -> Result<Connection, ErroApp> {
    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta).map_err(|e| {
            ErroApp::conexao_perdida()
                .com_detalhe(format!("não foi possível acessar {}: {e}", pasta.display()))
        })?;
    }

    let conexao = Connection::open(caminho).map_err(|e| {
        ErroApp::conexao_perdida().com_detalhe(format!("ao abrir {}: {e}", caminho.display()))
    })?;
    aplicar_pragmas(&conexao)?;
    Ok(conexao)
}

/// Só para teste. Em memória `journal_mode = TRUNCATE` não se aplica, então os
/// testes que dependem dos PRAGMA usam arquivo de verdade.
#[cfg(test)]
pub(crate) fn abrir_em_memoria() -> Result<Connection, ErroApp> {
    let conexao = Connection::open_in_memory()?;
    conexao.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conexao)
}

pub fn integridade_ok(conexao: &Connection) -> Result<bool, ErroApp> {
    let resultado: String = conexao.query_row("PRAGMA integrity_check", [], |l| l.get(0))?;
    Ok(resultado == "ok")
}

#[cfg(test)]
pub(crate) mod testes {
    use super::*;

    #[test]
    fn os_quatro_pragmas_ficam_ativos_na_conexao() {
        let pasta = tempdir();
        let conexao = abrir(&pasta.join("controle.db")).expect("abre");

        let journal: String = conexao
            .query_row("PRAGMA journal_mode", [], |l| l.get(0))
            .expect("lê journal_mode");
        let sincronismo: i64 = conexao
            .query_row("PRAGMA synchronous", [], |l| l.get(0))
            .expect("lê synchronous");
        let fks: i64 = conexao
            .query_row("PRAGMA foreign_keys", [], |l| l.get(0))
            .expect("lê foreign_keys");
        let espera: i64 = conexao
            .query_row("PRAGMA busy_timeout", [], |l| l.get(0))
            .expect("lê busy_timeout");

        assert_eq!(journal.to_lowercase(), "truncate");
        assert_eq!(sincronismo, 2, "synchronous = FULL");
        assert_eq!(fks, 1);
        assert_eq!(espera, 5_000);

        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn integridade_de_banco_novo_passa() {
        let pasta = tempdir();
        let conexao = abrir(&pasta.join("controle.db")).expect("abre");
        assert!(integridade_ok(&conexao).expect("roda integrity_check"));
        let _ = std::fs::remove_dir_all(&pasta);
    }

    pub(crate) fn tempdir() -> std::path::PathBuf {
        let unico = format!(
            "controle-teste-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        let caminho = std::env::temp_dir().join(unico);
        std::fs::create_dir_all(&caminho).expect("cria pasta temporária");
        caminho
    }
}
