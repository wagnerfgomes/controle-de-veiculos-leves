//! Trilha de auditoria.
//!
//! Grava **dentro da transação do chamador**, nunca numa conexão própria: um
//! registro que entrou e uma auditoria que não entrou é pior que nenhuma das
//! duas, porque dá a impressão de que a trilha é confiável.

use rusqlite::{params, Connection};

use crate::erro::ErroApp;
use crate::identidade;

pub const ABRIR_SAIDA: &str = "ABRIR_SAIDA";
pub const ENCERRAR_SAIDA: &str = "ENCERRAR_SAIDA";
pub const EDITAR_SAIDA: &str = "EDITAR_SAIDA";
pub const EXCLUIR_SAIDA: &str = "EXCLUIR_SAIDA";
pub const CRIAR: &str = "CRIAR";
pub const ATUALIZAR: &str = "ATUALIZAR";
pub const INATIVAR: &str = "INATIVAR";
pub const FUNDIR: &str = "FUNDIR";
pub const TOMADA_LOCK: &str = "TOMADA_LOCK";

pub struct Evento<'a> {
    pub entidade: &'a str,
    pub entidade_id: Option<i64>,
    pub acao: &'a str,
    pub antes: Option<String>,
    pub depois: Option<String>,
}

impl<'a> Evento<'a> {
    pub fn novo(entidade: &'a str, acao: &'a str) -> Self {
        Self {
            entidade,
            entidade_id: None,
            acao,
            antes: None,
            depois: None,
        }
    }

    pub fn id(mut self, id: i64) -> Self {
        self.entidade_id = Some(id);
        self
    }

    pub fn antes(mut self, valor: impl serde::Serialize) -> Self {
        self.antes = serde_json::to_string(&valor).ok();
        self
    }

    pub fn depois(mut self, valor: impl serde::Serialize) -> Self {
        self.depois = serde_json::to_string(&valor).ok();
        self
    }
}

pub fn registrar(conexao: &Connection, evento: Evento<'_>) -> Result<(), ErroApp> {
    conexao.execute(
        "INSERT INTO auditoria
             (entidade, entidade_id, acao, antes, depois, usuario_windows, maquina, em)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            evento.entidade,
            evento.entidade_id,
            evento.acao,
            evento.antes,
            evento.depois,
            identidade::usuario(),
            identidade::maquina(),
            crate::dominio::datahora::agora(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::db::{conexao, migracoes};

    #[test]
    fn evento_grava_usuario_maquina_e_json() {
        let c = conexao::abrir_em_memoria().expect("abre");
        migracoes::aplicar(&c, |_| Ok(())).expect("migra");

        registrar(
            &c,
            Evento::novo("saidas", ABRIR_SAIDA)
                .id(7)
                .depois(serde_json::json!({ "veiculo_id": 1 })),
        )
        .expect("grava auditoria");

        let (entidade, acao, depois, usuario): (String, String, Option<String>, String) = c
            .query_row(
                "SELECT entidade, acao, depois, usuario_windows FROM auditoria",
                [],
                |l| Ok((l.get(0)?, l.get(1)?, l.get(2)?, l.get(3)?)),
            )
            .expect("lê auditoria");

        assert_eq!(entidade, "saidas");
        assert_eq!(acao, ABRIR_SAIDA);
        assert_eq!(depois.as_deref(), Some(r#"{"veiculo_id":1}"#));
        assert!(!usuario.is_empty());
    }
}
