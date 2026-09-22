//! Migrações versionadas, aplicadas na abertura da conexão.
//!
//! O nível aplicado mora em `PRAGMA user_version` e **só lá**. Duplicá-lo numa
//! chave de `config` garante duas cópias divergindo na primeira migração escrita
//! com pressa, e `user_version` é a que o SQLite honra.

use rusqlite::Connection;

use crate::erro::ErroApp;
use crate::registro;

/// A base nasce vazia e para em 1: não existe seed. Importar a lista de frotas
/// do sistema antigo importaria também o lixo dele, que é justamente o que este
/// projeto existe para não repetir.
const MIGRACOES: &[(i64, &str, &str)] = &[(
    1,
    "001_inicial.sql",
    include_str!("../../migracoes/001_inicial.sql"),
)];

pub fn nivel_atual(conexao: &Connection) -> Result<i64, ErroApp> {
    Ok(conexao.query_row("PRAGMA user_version", [], |l| l.get(0))?)
}

/// Aplica o que faltar, em ordem. `antes_de_migrar` roda uma vez por migração
/// pendente, antes dela: é onde o backup entra. Migração sem backup é a que não
/// tem volta quando o SQL estava errado.
pub fn aplicar<F>(conexao: &Connection, mut antes_de_migrar: F) -> Result<i64, ErroApp>
where
    F: FnMut(i64) -> Result<(), ErroApp>,
{
    let mut atual = nivel_atual(conexao)?;

    for (numero, arquivo, sql) in MIGRACOES {
        if *numero <= atual {
            continue;
        }

        antes_de_migrar(atual)?;
        registro::info(&format!("aplicando migração {arquivo}"));

        conexao.execute_batch("BEGIN")?;
        if let Err(e) = conexao.execute_batch(sql) {
            let _ = conexao.execute_batch("ROLLBACK");
            return Err(ErroApp::from(e).com_detalhe(format!("migração {arquivo}")));
        }
        // Redundante quando o arquivo já traz o PRAGMA, e é de propósito: uma
        // migração futura que esqueça a linha não deixa o nível para trás.
        conexao.execute_batch(&format!("PRAGMA user_version = {numero}"))?;
        conexao.execute_batch("COMMIT")?;

        atual = *numero;
    }

    Ok(atual)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::db::conexao;

    #[test]
    fn schema_vazio_sobe_para_v1_com_tabelas_e_indices() {
        let c = conexao::abrir_em_memoria().expect("abre");
        assert_eq!(nivel_atual(&c).expect("lê nível"), 0);

        let nivel = aplicar(&c, |_| Ok(())).expect("migra");
        assert_eq!(nivel, 1);
        assert_eq!(nivel_atual(&c).expect("lê nível"), 1);

        for tabela in ["motoristas", "veiculos", "saidas", "auditoria", "config"] {
            let existe: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [tabela],
                    |l| l.get(0),
                )
                .expect("consulta sqlite_master");
            assert_eq!(existe, 1, "tabela {tabela} não foi criada");
        }

        for indice in ["ux_veiculo_em_uso", "ux_motorista_em_uso"] {
            let sql: String = c
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type='index' AND name=?1",
                    [indice],
                    |l| l.get(0),
                )
                .unwrap_or_else(|_| panic!("índice {indice} não foi criado"));
            assert!(
                sql.contains("WHERE dt_chegada IS NULL AND excluida_em IS NULL"),
                "índice {indice} não é parcial: {sql}"
            );
        }
    }

    #[test]
    fn migrar_duas_vezes_nao_reaplica() {
        let c = conexao::abrir_em_memoria().expect("abre");
        aplicar(&c, |_| Ok(())).expect("migra");

        let mut chamadas = 0;
        let nivel = aplicar(&c, |_| {
            chamadas += 1;
            Ok(())
        })
        .expect("migra de novo");

        assert_eq!(nivel, 1);
        assert_eq!(chamadas, 0, "não deve pedir backup sem migração pendente");
    }

    #[test]
    fn base_nova_nasce_sem_nenhum_cadastro() {
        let c = conexao::abrir_em_memoria().expect("abre");
        aplicar(&c, |_| Ok(())).expect("migra");

        for tabela in ["motoristas", "veiculos", "saidas"] {
            let total: i64 = c
                .query_row(&format!("SELECT COUNT(*) FROM {tabela}"), [], |l| l.get(0))
                .expect("conta");
            assert_eq!(total, 0, "{tabela} deveria nascer vazia");
        }

        let corte: String = c
            .query_row(
                "SELECT valor FROM config WHERE chave = 'data_corte'",
                [],
                |l| l.get(0),
            )
            .expect("lê data_corte");
        assert_eq!(corte.len(), 10, "data_corte deve ser AAAA-MM-DD: {corte}");
    }

    #[test]
    fn backup_e_pedido_antes_de_cada_migracao_pendente() {
        let c = conexao::abrir_em_memoria().expect("abre");
        let mut niveis = Vec::new();
        aplicar(&c, |nivel| {
            niveis.push(nivel);
            Ok(())
        })
        .expect("migra");
        assert_eq!(niveis, vec![0]);
    }
}
