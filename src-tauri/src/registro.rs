//! Log em arquivo, na máquina do usuário.
//!
//! É a única coisa que o app escreve fora da pasta de rede. Falha de log nunca
//! derruba nada: sem lugar para escrever, a linha se perde e o app segue.

use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

static ARQUIVO: OnceLock<Option<PathBuf>> = OnceLock::new();

fn caminho() -> Option<&'static PathBuf> {
    ARQUIVO
        .get_or_init(|| {
            let base = if cfg!(windows) {
                std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
            } else {
                std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
                    })
            }?;
            let pasta = base.join("ControleVeiculos");
            std::fs::create_dir_all(&pasta).ok()?;
            Some(pasta.join("app.log"))
        })
        .as_ref()
}

fn escrever(nivel: &str, texto: &str) {
    let linha = format!(
        "{} [{}] {}\n",
        crate::dominio::datahora::agora_com_segundos(),
        nivel,
        texto
    );
    eprint!("{linha}");

    let Some(arquivo) = caminho() else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(arquivo)
    {
        let _ = f.write_all(linha.as_bytes());
    }
}

pub fn info(texto: &str) {
    escrever("INFO", texto)
}

pub fn aviso(texto: &str) {
    escrever("AVISO", texto)
}

pub fn erro(texto: &str) {
    escrever("ERRO", texto)
}

/// Caminho do log, para a tela de erro conseguir dizer onde olhar.
pub fn caminho_log() -> Option<String> {
    caminho().map(|c| c.display().to_string())
}
