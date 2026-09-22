//! Leitura do `config.toml` que mora ao lado do executável.
//!
//! O caminho da pasta de rede nunca é compilado no binário: trocar de servidor
//! precisa ser editar um arquivo de texto, não recompilar.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::erro::ErroApp;
use crate::registro;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modo {
    Producao,
    Demonstracao,
}

#[derive(Debug, Deserialize)]
struct ConfigArquivo {
    caminho_dados: String,
    #[serde(default = "modo_padrao")]
    modo: Modo,
}

fn modo_padrao() -> Modo {
    Modo::Producao
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Raiz que contém `dados/`, `backups/` e `relatorios/`.
    pub raiz: PathBuf,
    pub modo: Modo,
    /// De onde a configuração veio, para a mensagem de erro dizer o caminho tentado.
    pub origem: String,
}

impl Config {
    pub fn dados(&self) -> PathBuf {
        self.raiz.join("dados")
    }

    pub fn banco(&self) -> PathBuf {
        self.dados().join("controle.db")
    }

    pub fn lock(&self) -> PathBuf {
        self.dados().join("controle.lock")
    }

    pub fn backups(&self) -> PathBuf {
        self.raiz.join("backups")
    }

    pub fn relatorios(&self) -> PathBuf {
        self.raiz.join("relatorios")
    }

    pub fn demonstracao(&self) -> bool {
        self.modo == Modo::Demonstracao
    }
}

/// Pasta onde o executável está. É a âncora de tudo: `config.toml`, e em
/// demonstração também o próprio banco.
fn pasta_do_executavel() -> Result<PathBuf, ErroApp> {
    let exe = std::env::current_exe()
        .map_err(|e| ErroApp::validacao(format!("não foi possível localizar o executável: {e}")))?;
    let pasta = exe
        .parent()
        .ok_or_else(|| ErroApp::validacao("executável sem pasta pai"))?;
    Ok(pasta.to_path_buf())
}

pub fn carregar() -> Result<Config, ErroApp> {
    let pasta = pasta_do_executavel()?;

    // A feature `demo` não consulta config.toml: o modo é decidido na compilação,
    // e por isso não há como apontar a apresentação para dado real por engano.
    if cfg!(feature = "demo") {
        return Ok(Config {
            raiz: pasta.join("dados-demo"),
            modo: Modo::Demonstracao,
            origem: "feature demo (compilada)".to_string(),
        });
    }

    let arquivo = pasta.join("config.toml");
    if !arquivo.exists() {
        return Ok(config_desenvolvimento(&arquivo));
    }

    let texto = std::fs::read_to_string(&arquivo).map_err(|e| {
        ErroApp::validacao(format!("não foi possível ler {}: {e}", arquivo.display()))
    })?;
    let lido: ConfigArquivo = toml::from_str(&texto).map_err(|e| {
        ErroApp::validacao(format!("config.toml inválido ({}): {e}", arquivo.display()))
    })?;

    let raiz = if lido.modo == Modo::Demonstracao {
        pasta.join("dados-demo")
    } else {
        PathBuf::from(&lido.caminho_dados)
    };

    Ok(Config {
        raiz,
        modo: lido.modo,
        origem: arquivo.display().to_string(),
    })
}

/// Sem `config.toml` ao lado do executável, o app está rodando de
/// `cargo run` ou `tauri dev`. Cai numa pasta local em modo produção: o resto
/// do código não pode saber a diferença entre esta pasta e um caminho UNC.
fn config_desenvolvimento(tentado: &Path) -> Config {
    registro::aviso(&format!(
        "config.toml não encontrado em {}; usando pasta local de desenvolvimento",
        tentado.display()
    ));
    Config {
        raiz: PathBuf::from("dados-dev"),
        modo: Modo::Producao,
        origem: format!("padrão de desenvolvimento (nenhum {})", tentado.display()),
    }
}
