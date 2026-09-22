//! Abertura da sessão, tela de bloqueio e tomada de posse.

use serde::Serialize;
use tauri::State;

use crate::db::auditoria::{self, Evento};
use crate::db::backup;
use crate::dominio::datahora;
use crate::erro::ErroApp;
use crate::estado::Estado;
use crate::identidade;
use crate::lock::{Lock, LockAtual};
use crate::registro;
use crate::sessao::EstadoSessao;

/// O que a tela de bloqueio mostra. Sem menu e sem contorno: o caminho certo é
/// procurar a pessoa, não insistir no botão.
#[derive(Debug, Clone, Serialize)]
pub struct InfoBloqueio {
    pub usuario_windows: String,
    pub maquina: String,
    pub desde: String,
    pub minutos_sem_heartbeat: Option<i64>,
    /// Só com heartbeat parado há mais de 10 minutos.
    pub pode_tomar_posse: bool,
}

#[tauri::command]
pub fn iniciar_sessao(estado: State<'_, Estado>) -> Result<EstadoSessao, ErroApp> {
    let sessao = estado.abrir()?;
    sessao.estado()
}

#[tauri::command]
pub fn estado_sessao(estado: State<'_, Estado>) -> Result<EstadoSessao, ErroApp> {
    estado.sessao()?.estado()
}

#[tauri::command]
pub fn info_bloqueio(estado: State<'_, Estado>) -> Result<Option<InfoBloqueio>, ErroApp> {
    let Some(info) = LockAtual::info(&estado.config.lock())? else {
        return Ok(None);
    };

    Ok(Some(InfoBloqueio {
        usuario_windows: info.usuario_windows.clone(),
        maquina: info.maquina.clone(),
        desde: info.hora_de_inicio(),
        minutos_sem_heartbeat: info.minutos_desde_heartbeat(),
        pode_tomar_posse: info.pode_tomar_posse(),
    }))
}

/// Exige digitar `CONFIRMAR`. Sem esse atrito, e sem o atraso de 10 minutos no
/// heartbeat, tomada de posse vira o botão que todo mundo aperta, e duas
/// máquinas escrevendo no mesmo arquivo é como o banco corrompe.
#[tauri::command]
pub fn tomar_posse_lock(
    estado: State<'_, Estado>,
    confirmacao: String,
) -> Result<EstadoSessao, ErroApp> {
    if confirmacao.trim() != "CONFIRMAR" {
        return Err(ErroApp::validacao(
            "Digite CONFIRMAR para assumir a sessão.",
        ));
    }

    let caminho_lock = estado.config.lock();
    let anterior = LockAtual::info(&caminho_lock)?;

    match &anterior {
        Some(info) if !info.pode_tomar_posse() => {
            return Err(ErroApp::validacao(
                "A outra sessão ainda está ativa. Procure a pessoa antes de assumir.",
            ));
        }
        _ => {}
    }

    registro::aviso(&format!(
        "tomada de posse do lock por {} em {}",
        identidade::usuario(),
        identidade::maquina()
    ));

    let sessao = estado.abrir()?;
    let conexao = sessao.travar_conexao()?;
    auditoria::registrar(
        &conexao,
        Evento::novo("sessao", auditoria::TOMADA_LOCK)
            .antes(&anterior)
            .depois(serde_json::json!({
                "usuario_windows": identidade::usuario(),
                "maquina": identidade::maquina(),
                "em": datahora::agora(),
            })),
    )?;
    drop(conexao);

    sessao.estado()
}

#[tauri::command]
pub fn forcar_backup(estado: State<'_, Estado>) -> Result<String, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;

    let conexao = sessao.travar_conexao()?;
    let caminho = backup::gerar(&conexao, &sessao.config.backups())?;
    conexao.execute(
        "UPDATE config SET valor = ?1 WHERE chave = 'ultimo_backup_em'",
        [datahora::agora()],
    )?;
    drop(conexao);

    backup::rotacionar(&sessao.config.backups())?;
    Ok(caminho.display().to_string())
}

#[tauri::command]
pub fn encerrar_sessao(estado: State<'_, Estado>) -> Result<(), ErroApp> {
    estado.fechar();
    Ok(())
}
