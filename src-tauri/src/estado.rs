//! Estado global do app.
//!
//! A sessão é **opcional** de propósito: quando o lock está tomado por outra
//! máquina, o app precisa existir para mostrar a tela de bloqueio. Exigir sessão
//! aberta para o app subir deixaria o usuário olhando para uma janela morta sem
//! saber quem está com o sistema.

use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::erro::ErroApp;
use crate::sessao::Sessao;

pub struct Estado {
    pub config: Config,
    sessao: Mutex<Option<Arc<Sessao>>>,
}

impl Estado {
    pub fn novo(config: Config) -> Self {
        Self {
            config,
            sessao: Mutex::new(None),
        }
    }

    pub fn sessao(&self) -> Result<Arc<Sessao>, ErroApp> {
        let guarda = self
            .sessao
            .lock()
            .map_err(|_| ErroApp::banco("o estado da sessão ficou inconsistente"))?;
        guarda.clone().ok_or_else(ErroApp::sessao_perdida)
    }

    pub fn abrir(&self) -> Result<Arc<Sessao>, ErroApp> {
        if let Ok(guarda) = self.sessao.lock() {
            if let Some(ja_aberta) = guarda.clone() {
                return Ok(ja_aberta);
            }
        }

        let nova = Arc::new(Sessao::abrir(self.config.clone())?);

        let mut guarda = self
            .sessao
            .lock()
            .map_err(|_| ErroApp::banco("o estado da sessão ficou inconsistente"))?;
        *guarda = Some(nova.clone());
        Ok(nova)
    }

    pub fn fechar(&self) {
        let Ok(mut guarda) = self.sessao.lock() else {
            return;
        };
        if let Some(sessao) = guarda.take() {
            sessao.fechar();
        }
    }
}
