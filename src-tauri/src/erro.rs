//! Erro único da aplicação.
//!
//! O React trata pelo `codigo`, nunca pela mensagem: mensagem é para humano e
//! muda; código é contrato.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ErroApp {
    pub codigo: String,
    pub mensagem: String,
    pub detalhe: Option<String>,
}

impl std::fmt::Display for ErroApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.codigo, self.mensagem)?;
        if let Some(d) = &self.detalhe {
            write!(f, " ({d})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ErroApp {}

impl ErroApp {
    fn novo(codigo: &str, mensagem: impl Into<String>) -> Self {
        Self {
            codigo: codigo.to_string(),
            mensagem: mensagem.into(),
            detalhe: None,
        }
    }

    pub fn com_detalhe(mut self, detalhe: impl Into<String>) -> Self {
        self.detalhe = Some(detalhe.into());
        self
    }

    pub fn veiculo_em_uso() -> Self {
        Self::novo(
            "VEICULO_EM_USO",
            "Este veículo já está em viagem e ainda não retornou.",
        )
    }

    pub fn motorista_em_uso() -> Self {
        Self::novo(
            "MOTORISTA_EM_USO",
            "Este condutor já está em viagem e ainda não retornou.",
        )
    }

    pub fn duplicado(mensagem: impl Into<String>) -> Self {
        Self::novo("DUPLICADO", mensagem)
    }

    pub fn semelhante(mensagem: impl Into<String>) -> Self {
        Self::novo("SEMELHANTE", mensagem)
    }

    pub fn nao_encontrado(mensagem: impl Into<String>) -> Self {
        Self::novo("NAO_ENCONTRADO", mensagem)
    }

    pub fn validacao(mensagem: impl Into<String>) -> Self {
        Self::novo("VALIDACAO", mensagem)
    }

    pub fn banco(mensagem: impl Into<String>) -> Self {
        Self::novo("BANCO", mensagem)
    }

    pub fn sessao_perdida() -> Self {
        Self::novo(
            "SESSAO_PERDIDA",
            "A sessão com o servidor foi perdida e o registro não pode ser gravado. \
             Reconecte antes de continuar.",
        )
    }

    pub fn conexao_perdida() -> Self {
        Self::novo(
            "CONEXAO_PERDIDA",
            "Conexão com o servidor perdida, o último registro pode não ter sido salvo.",
        )
    }
}

/// Traduz a falha do SQLite para o vocabulário da aplicação.
///
/// As duas regras de ouro são garantidas por índice único parcial, então a
/// violação chega aqui como erro de banco. Deixá-la vazar como `BANCO` daria ao
/// operador "erro de banco de dados" no lugar de "o carro está na rua com
/// fulano", que é a informação de que ele precisa.
impl From<rusqlite::Error> for ErroApp {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ffi, msg) = &e {
            let texto = msg.clone().unwrap_or_default();

            if ffi.code == rusqlite::ErrorCode::ConstraintViolation {
                if let Some(mapeado) = por_restricao(&texto) {
                    return mapeado;
                }
            }

            // Erro de I/O sobre SMB é o servidor de arquivos sumindo, não um
            // banco quebrado. A tela oferece reconectar em vez de assustar.
            if matches!(
                ffi.code,
                rusqlite::ErrorCode::CannotOpen
                    | rusqlite::ErrorCode::SystemIoFailure
                    | rusqlite::ErrorCode::DatabaseBusy
                    | rusqlite::ErrorCode::DatabaseLocked
            ) {
                return ErroApp::conexao_perdida().com_detalhe(texto);
            }
        }

        ErroApp::banco("Falha ao acessar o banco de dados.").com_detalhe(e.to_string())
    }
}

/// Ao violar um índice único **parcial**, o SQLite não cita o nome do índice:
/// a mensagem é `UNIQUE constraint failed: saidas.veiculo_id`, igual à de uma
/// restrição de coluna. Confirmado contra o schema deste projeto; casar pelo
/// nome `ux_veiculo_em_uso` sozinho deixaria o erro vazar como `BANCO`.
///
/// `saidas.veiculo_id` e `saidas.motorista_id` só são únicos por causa desses
/// índices, então a coluna identifica a regra sem ambiguidade. O nome do índice
/// continua sendo testado primeiro, para o caso de uma versão do SQLite passar
/// a reportá-lo.
fn por_restricao(texto: &str) -> Option<ErroApp> {
    if texto.contains("ux_veiculo_em_uso") || texto.contains("saidas.veiculo_id") {
        return Some(ErroApp::veiculo_em_uso());
    }
    if texto.contains("ux_motorista_em_uso") || texto.contains("saidas.motorista_id") {
        return Some(ErroApp::motorista_em_uso());
    }
    if texto.contains("motoristas.nome_norm") {
        return Some(ErroApp::duplicado("Já existe um condutor com esse nome."));
    }
    if texto.contains("motoristas.matricula") {
        return Some(ErroApp::duplicado(
            "Já existe um condutor com essa matrícula.",
        ));
    }
    if texto.contains("veiculos.frota") {
        return Some(ErroApp::duplicado("Já existe um veículo com essa frota."));
    }
    if texto.contains("veiculos.placa") {
        return Some(ErroApp::duplicado("Já existe um veículo com essa placa."));
    }
    None
}

#[cfg(test)]
mod testes {
    use super::*;
    use rusqlite::ffi;

    fn falha_unique(msg: &str) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(
            ffi::Error {
                code: rusqlite::ErrorCode::ConstraintViolation,
                extended_code: ffi::SQLITE_CONSTRAINT_UNIQUE,
            },
            Some(msg.to_string()),
        )
    }

    /// Esta é a mensagem que o SQLite realmente produz ao violar
    /// `ux_veiculo_em_uso`: sem o nome do índice.
    #[test]
    fn coluna_de_veiculo_vira_veiculo_em_uso() {
        let erro = ErroApp::from(falha_unique("UNIQUE constraint failed: saidas.veiculo_id"));
        assert_eq!(erro.codigo, "VEICULO_EM_USO");
    }

    #[test]
    fn coluna_de_motorista_vira_motorista_em_uso() {
        let erro = ErroApp::from(falha_unique(
            "UNIQUE constraint failed: saidas.motorista_id",
        ));
        assert_eq!(erro.codigo, "MOTORISTA_EM_USO");
    }

    #[test]
    fn o_nome_do_indice_tambem_e_aceito() {
        let erro = ErroApp::from(falha_unique(
            "UNIQUE constraint failed: index 'ux_veiculo_em_uso'",
        ));
        assert_eq!(erro.codigo, "VEICULO_EM_USO");
    }

    #[test]
    fn nome_norm_repetido_vira_duplicado() {
        let erro = ErroApp::from(falha_unique(
            "UNIQUE constraint failed: motoristas.nome_norm",
        ));
        assert_eq!(erro.codigo, "DUPLICADO");
    }

    #[test]
    fn erro_de_io_vira_conexao_perdida() {
        let erro = ErroApp::from(rusqlite::Error::SqliteFailure(
            ffi::Error {
                code: rusqlite::ErrorCode::SystemIoFailure,
                extended_code: ffi::SQLITE_IOERR,
            },
            Some("disk I/O error".to_string()),
        ));
        assert_eq!(erro.codigo, "CONEXAO_PERDIDA");
    }

    #[test]
    fn falha_desconhecida_ainda_vira_banco() {
        let erro = ErroApp::from(rusqlite::Error::QueryReturnedNoRows);
        assert_eq!(erro.codigo, "BANCO");
    }
}
