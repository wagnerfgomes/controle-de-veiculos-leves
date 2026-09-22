//! Um usuário por vez.
//!
//! É o lock que torna segura a decisão de manter o banco direto na pasta de
//! rede: quase toda corrupção de SQLite sobre SMB vem de escrita concorrente, e
//! aqui ela não existe.
//!
//! O handle fica vivo na struct de estado pela sessão inteira. O sistema
//! operacional o libera sozinho se o processo morrer, e é isso que evita lock
//! órfão em travamento, `Fim de tarefa` ou queda de energia da estação.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::erro::ErroApp;
use crate::identidade;
use crate::registro;

pub const INTERVALO_HEARTBEAT_SEGUNDOS: u64 = 30;
/// Sem esse atraso, tomada de posse vira o botão que todo mundo aperta.
pub const IDADE_PARA_TOMADA_POSSE_MINUTOS: i64 = 10;
pub const FALHAS_PARA_DEGRADAR: u32 = 3;

/// O SPEC fixa este formato para o `controle.lock.info`, com segundos, e ele é
/// deliberadamente diferente do `YYYY-MM-DD HH:MM` do banco: aqui a diferença de
/// segundos entre dois heartbeats é a informação, não ruído.
const FORMATO_INFO: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoLock {
    pub usuario_windows: String,
    pub maquina: String,
    pub pid: u32,
    pub inicio: String,
    pub heartbeat: String,
}

impl InfoLock {
    fn nova() -> Self {
        let agora = chrono::Local::now().format(FORMATO_INFO).to_string();
        Self {
            usuario_windows: identidade::usuario(),
            maquina: identidade::maquina(),
            pid: std::process::id(),
            inicio: agora.clone(),
            heartbeat: agora,
        }
    }

    fn analisar(texto: &str) -> Option<NaiveDateTime> {
        NaiveDateTime::parse_from_str(texto, FORMATO_INFO).ok()
    }

    /// Minutos desde o último heartbeat. `None` quando o carimbo é ilegível: aí
    /// a tomada de posse **não** é oferecida, porque não dá para afirmar que o
    /// dono sumiu.
    pub fn minutos_desde_heartbeat(&self) -> Option<i64> {
        let batida = Self::analisar(&self.heartbeat)?;
        Some((chrono::Local::now().naive_local() - batida).num_minutes())
    }

    pub fn pode_tomar_posse(&self) -> bool {
        self.minutos_desde_heartbeat()
            .is_some_and(|m| m > IDADE_PARA_TOMADA_POSSE_MINUTOS)
    }

    /// `08:12`, como a tela de bloqueio mostra.
    pub fn hora_de_inicio(&self) -> String {
        Self::analisar(&self.inicio)
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_else(|| self.inicio.clone())
    }
}

pub trait Lock: Sized {
    fn adquirir(caminho: &Path) -> Result<Self, ErroApp>;
    fn heartbeat(&self) -> Result<(), ErroApp>;
    fn info(caminho: &Path) -> Result<Option<InfoLock>, ErroApp>;
    fn liberar(self);
}

fn caminho_info(caminho_lock: &Path) -> PathBuf {
    let mut nome = caminho_lock.as_os_str().to_os_string();
    nome.push(".info");
    PathBuf::from(nome)
}

/// `controle.lock.info` é arquivo separado porque o handle exclusivo bloqueia
/// até a leitura do `controle.lock`: sem ele a tela de bloqueio não teria o que
/// mostrar além de "em uso".
fn escrever_info(caminho_lock: &Path, info: &InfoLock) -> Result<(), ErroApp> {
    let json = serde_json::to_string(info)
        .map_err(|e| ErroApp::banco("falha ao serializar o lock").com_detalhe(e.to_string()))?;

    let caminho = caminho_info(caminho_lock);
    let mut arquivo = File::create(&caminho).map_err(|e| {
        ErroApp::conexao_perdida().com_detalhe(format!("ao escrever {}: {e}", caminho.display()))
    })?;
    arquivo.write_all(json.as_bytes()).map_err(|e| {
        ErroApp::conexao_perdida().com_detalhe(format!("ao escrever {}: {e}", caminho.display()))
    })?;
    Ok(())
}

fn ler_info(caminho_lock: &Path) -> Result<Option<InfoLock>, ErroApp> {
    let caminho = caminho_info(caminho_lock);
    let Ok(texto) = std::fs::read_to_string(&caminho) else {
        return Ok(None);
    };
    // Info corrompido não é motivo para derrubar o app: a tela de bloqueio
    // aparece sem o nome de quem está usando, que é pior mas não fatal.
    match serde_json::from_str(&texto) {
        Ok(info) => Ok(Some(info)),
        Err(e) => {
            registro::aviso(&format!("{} ilegível: {e}", caminho.display()));
            Ok(None)
        }
    }
}

fn remover_info(caminho_lock: &Path) {
    let caminho = caminho_info(caminho_lock);
    if let Err(e) = std::fs::remove_file(&caminho) {
        if e.kind() != std::io::ErrorKind::NotFound {
            registro::aviso(&format!(
                "não foi possível remover {}: {e}",
                caminho.display()
            ));
        }
    }
}

fn preparar_pasta(caminho_lock: &Path) -> Result<(), ErroApp> {
    let Some(pasta) = caminho_lock.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(pasta).map_err(|e| {
        ErroApp::conexao_perdida()
            .com_detalhe(format!("não foi possível acessar {}: {e}", pasta.display()))
    })
}

fn erro_em_uso(caminho_lock: &Path) -> ErroApp {
    let info = ler_info(caminho_lock).ok().flatten();
    let detalhe = match &info {
        Some(i) => format!(
            "{} · {} · desde {}",
            i.usuario_windows,
            i.maquina,
            i.hora_de_inicio()
        ),
        None => "não foi possível identificar quem está usando".to_string(),
    };
    ErroApp::validacao("O sistema já está aberto em outra máquina.")
        .com_detalhe(format!("EM_USO:{detalhe}"))
}

// ---------------------------------------------------------------- Windows ---

#[cfg(windows)]
#[derive(Debug)]
pub struct LockWindows {
    _arquivo: File,
    caminho: PathBuf,
    inicio: String,
}

#[cfg(windows)]
impl Lock for LockWindows {
    fn adquirir(caminho: &Path) -> Result<Self, ErroApp> {
        use std::os::windows::fs::OpenOptionsExt;

        preparar_pasta(caminho)?;

        let arquivo = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0) // FILE_SHARE_NONE
            .open(caminho)
            .map_err(|_| erro_em_uso(caminho))?;

        let info = InfoLock::nova();
        escrever_info(caminho, &info)?;

        Ok(Self {
            _arquivo: arquivo,
            caminho: caminho.to_path_buf(),
            inicio: info.inicio,
        })
    }

    fn heartbeat(&self) -> Result<(), ErroApp> {
        let mut info = InfoLock::nova();
        info.inicio = self.inicio.clone();
        escrever_info(&self.caminho, &info)
    }

    fn info(caminho: &Path) -> Result<Option<InfoLock>, ErroApp> {
        ler_info(caminho)
    }

    fn liberar(self) {
        remover_info(&self.caminho);
    }
}

// ------------------------------------------------------------------- Unix ---

/// `flock` advisory protege contra outra instância **na mesma máquina** e nada
/// além disso. Não é equivalente ao handle exclusivo do Windows, e por isso
/// nenhum teste de concorrência rodado em Linux conta para os critérios de
/// aceite. Existe só para o desenvolvimento e para a apresentação rodarem.
#[cfg(unix)]
#[derive(Debug)]
pub struct LockDesenvolvimento {
    arquivo: File,
    caminho: PathBuf,
    inicio: String,
}

#[cfg(unix)]
impl Lock for LockDesenvolvimento {
    fn adquirir(caminho: &Path) -> Result<Self, ErroApp> {
        use std::os::unix::io::AsRawFd;

        preparar_pasta(caminho)?;
        registro::aviso(
            "LOCK EM MODO DE DESENVOLVIMENTO: flock advisory, não o handle exclusivo do \
             Windows. Não protege contra outra máquina na rede.",
        );

        let arquivo = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(caminho)
            .map_err(|e| {
                ErroApp::conexao_perdida()
                    .com_detalhe(format!("ao abrir {}: {e}", caminho.display()))
            })?;

        // SAFETY: fd válido enquanto `arquivo` vive, e LOCK_NB não bloqueia.
        let obtido = unsafe { libc::flock(arquivo.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if obtido != 0 {
            return Err(erro_em_uso(caminho));
        }

        let info = InfoLock::nova();
        escrever_info(caminho, &info)?;

        Ok(Self {
            arquivo,
            caminho: caminho.to_path_buf(),
            inicio: info.inicio,
        })
    }

    fn heartbeat(&self) -> Result<(), ErroApp> {
        let mut info = InfoLock::nova();
        info.inicio = self.inicio.clone();
        escrever_info(&self.caminho, &info)
    }

    fn info(caminho: &Path) -> Result<Option<InfoLock>, ErroApp> {
        ler_info(caminho)
    }

    fn liberar(self) {
        use std::os::unix::io::AsRawFd;
        // SAFETY: fd ainda válido; o drop de `arquivo` logo em seguida também
        // liberaria, e soltar explicitamente deixa a ordem óbvia.
        unsafe {
            libc::flock(self.arquivo.as_raw_fd(), libc::LOCK_UN);
        }
        remover_info(&self.caminho);
    }
}

#[cfg(windows)]
pub type LockAtual = LockWindows;
#[cfg(unix)]
pub type LockAtual = LockDesenvolvimento;

/// O que a barra de status precisa saber para se denunciar em Linux.
pub const fn lock_e_confiavel() -> bool {
    cfg!(windows)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::db::conexao::testes::tempdir;

    #[test]
    fn segunda_aquisicao_na_mesma_maquina_e_recusada_com_quem_esta_usando() {
        let pasta = tempdir();
        let caminho = pasta.join("controle.lock");

        let primeiro = LockAtual::adquirir(&caminho).expect("primeiro adquire");
        let erro = LockAtual::adquirir(&caminho).expect_err("segundo deve falhar");

        let detalhe = erro.detalhe.unwrap_or_default();
        assert!(detalhe.starts_with("EM_USO:"), "detalhe: {detalhe}");
        assert!(detalhe.contains(&identidade::maquina()));

        primeiro.liberar();
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn liberar_permite_readquirir_e_apaga_o_info() {
        let pasta = tempdir();
        let caminho = pasta.join("controle.lock");

        let primeiro = LockAtual::adquirir(&caminho).expect("adquire");
        primeiro.liberar();

        assert!(!caminho_info(&caminho).exists(), "info deveria ter sumido");

        let segundo = LockAtual::adquirir(&caminho).expect("readquire depois de liberar");
        segundo.liberar();
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn heartbeat_preserva_o_inicio_e_avanca_a_batida() {
        let pasta = tempdir();
        let caminho = pasta.join("controle.lock");

        let lock = LockAtual::adquirir(&caminho).expect("adquire");
        let antes = LockAtual::info(&caminho).expect("lê").expect("existe");

        lock.heartbeat().expect("bate");
        let depois = LockAtual::info(&caminho).expect("lê").expect("existe");

        assert_eq!(antes.inicio, depois.inicio);
        assert!(depois.heartbeat >= antes.heartbeat);
        assert_eq!(depois.pid, std::process::id());

        lock.liberar();
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn tomada_de_posse_so_e_oferecida_com_heartbeat_parado() {
        let recente = InfoLock::nova();
        assert!(!recente.pode_tomar_posse());

        let mut antigo = InfoLock::nova();
        antigo.heartbeat = (chrono::Local::now() - chrono::Duration::minutes(11))
            .format(FORMATO_INFO)
            .to_string();
        assert!(antigo.pode_tomar_posse());

        let mut ilegivel = InfoLock::nova();
        ilegivel.heartbeat = "ontem de tarde".to_string();
        assert!(
            !ilegivel.pode_tomar_posse(),
            "carimbo ilegível não autoriza tomada de posse"
        );
    }

    #[test]
    fn info_ausente_nao_e_erro() {
        let pasta = tempdir();
        let caminho = pasta.join("controle.lock");
        assert!(LockAtual::info(&caminho).expect("lê").is_none());
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
