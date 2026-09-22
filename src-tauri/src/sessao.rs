//! Abertura e fechamento da sessão, e o estado que vive enquanto o app está no ar.
//!
//! A ordem da abertura é a da seção 4 do SPEC e não é negociável: o lock vem
//! **antes** da conexão. Abrir o banco primeiro e só então descobrir que outra
//! máquina está com ele é exatamente a escrita concorrente que o lock existe
//! para impedir.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;

use crate::config::{Config, Modo};
use crate::db::{backup, conexao, migracoes};
use crate::dominio::datahora;
use crate::erro::ErroApp;
use crate::identidade;
use crate::lock::{Lock, LockAtual};
use crate::registro;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Conexao {
    Normal,
    Degradada,
}

#[derive(Debug, Clone, Serialize)]
pub struct EstadoSessao {
    pub usuario_windows: String,
    pub maquina: String,
    pub inicio: String,
    pub conexao: Conexao,
    pub ultimo_backup: Option<String>,
    pub user_version: i64,
    pub caminho_dados: String,
    pub modo: Modo,
    /// Falso em Linux. A barra de status é obrigada a mostrar isso: stub
    /// silencioso é como se descobre em produção que o lock nunca existiu.
    pub lock_confiavel: bool,
}

pub struct Sessao {
    pub config: Config,
    pub conexao: Mutex<Connection>,
    pub lock: Mutex<Option<LockAtual>>,
    pub inicio: String,
    pub estado_conexao: Mutex<Conexao>,
    pub falhas_heartbeat: Mutex<u32>,
}

impl Sessao {
    /// Abre a sessão na ordem da seção 4. Devolve o erro já traduzido: a tela de
    /// bloqueio se distingue da tela de erro pelo `codigo` e pelo `detalhe`.
    pub fn abrir(config: Config) -> Result<Self, ErroApp> {
        registro::info(&format!(
            "abrindo sessão — modo {:?}, dados em {}, configuração de {}",
            config.modo,
            config.raiz.display(),
            config.origem
        ));

        // 2. O lock antes de tudo que toca o banco, e **antes** de recriar a
        //    base de demonstração: a segunda instância precisa cair na tela de
        //    bloqueio sem ter apagado o banco que a primeira está usando. Na
        //    apresentação é justamente isso que se mostra.
        let lock = LockAtual::adquirir(&config.lock())?;

        if config.demonstracao() {
            if let Err(e) = preparar_demonstracao(&config) {
                lock.liberar();
                return Err(e);
            }
        }

        // 3. Conexão sobre o arquivo da rede, com os quatro PRAGMA.
        let conexao = match conexao::abrir(&config.banco()) {
            Ok(c) => c,
            Err(e) => {
                lock.liberar();
                return Err(e);
            }
        };

        // 4. Integridade antes de escrever qualquer coisa. O desvio automático
        //    para `backups\corrompidos\` com oferta de restauração é da Fase 9;
        //    aqui a sessão apenas se recusa a abrir, que é o essencial: seguir
        //    gravando sobre um arquivo corrompido destrói o que ainda dava para
        //    recuperar.
        match conexao::integridade_ok(&conexao) {
            Ok(true) => {}
            Ok(false) => {
                lock.liberar();
                return Err(ErroApp::banco(
                    "O banco de dados está corrompido e o sistema não pode ser aberto. \
                     Procure o suporte antes de continuar.",
                )
                .com_detalhe(format!("ARQUIVO:{}", config.banco().display())));
            }
            Err(e) => {
                lock.liberar();
                return Err(e);
            }
        }

        // 5. Migrações pendentes, com backup antes de cada uma. Numa base que
        //    ainda não existe não há o que salvar, e o backup é pulado.
        let pasta_backups = config.backups();
        let resultado = migracoes::aplicar(&conexao, |nivel_atual| {
            if nivel_atual == 0 {
                return Ok(());
            }
            backup::gerar(&conexao, &pasta_backups).map(|_| ())
        });

        if let Err(e) = resultado {
            lock.liberar();
            return Err(e);
        }

        // A base de demonstração é recriada acima, então aqui ela está sempre
        // vazia e sempre precisa ser populada.
        if config.demonstracao() {
            if let Err(e) = crate::demo::popular(&conexao) {
                lock.liberar();
                return Err(e);
            }
        }

        Ok(Self {
            config,
            conexao: Mutex::new(conexao),
            lock: Mutex::new(Some(lock)),
            inicio: datahora::agora(),
            estado_conexao: Mutex::new(Conexao::Normal),
            falhas_heartbeat: Mutex::new(0),
        })
    }

    pub fn estado(&self) -> Result<EstadoSessao, ErroApp> {
        let conexao = self.travar_conexao()?;
        let user_version = migracoes::nivel_atual(&conexao)?;
        let ultimo_backup: Option<String> = conexao
            .query_row(
                "SELECT valor FROM config WHERE chave = 'ultimo_backup_em'",
                [],
                |l| l.get::<_, String>(0),
            )
            .ok()
            .filter(|v| !v.is_empty());

        Ok(EstadoSessao {
            usuario_windows: identidade::usuario(),
            maquina: identidade::maquina(),
            inicio: self.inicio.clone(),
            conexao: self.conexao_atual(),
            ultimo_backup,
            user_version,
            caminho_dados: self.config.raiz.display().to_string(),
            modo: self.config.modo,
            lock_confiavel: crate::lock::lock_e_confiavel(),
        })
    }

    pub fn conexao_atual(&self) -> Conexao {
        self.estado_conexao
            .lock()
            .map(|e| *e)
            .unwrap_or(Conexao::Degradada)
    }

    /// Todo comando que escreve passa por aqui antes. Em modo degradado o lock
    /// pode ter sido perdido, e gravar assim mesmo é como duas máquinas acabam
    /// escrevendo no mesmo arquivo.
    pub fn exigir_conexao_normal(&self) -> Result<(), ErroApp> {
        if self.conexao_atual() == Conexao::Degradada {
            return Err(ErroApp::sessao_perdida());
        }
        Ok(())
    }

    pub fn travar_conexao(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ErroApp> {
        self.conexao
            .lock()
            .map_err(|_| ErroApp::banco("a conexão ficou num estado inconsistente"))
    }

    pub fn fechar(&self) {
        registro::info("fechando sessão");

        if let Ok(conexao) = self.travar_conexao() {
            if !self.config.demonstracao() {
                match backup::gerar(&conexao, &self.config.backups()) {
                    Ok(_) => {
                        let _ = backup::rotacionar(&self.config.backups());
                        let _ = conexao.execute(
                            "UPDATE config SET valor = ?1 WHERE chave = 'ultimo_backup_em'",
                            [datahora::agora()],
                        );
                    }
                    Err(e) => registro::erro(&format!("backup de fechamento falhou: {e}")),
                }
            }
        }

        if let Ok(mut guarda) = self.lock.lock() {
            if let Some(lock) = guarda.take() {
                lock.liberar();
            }
        }
    }
}

/// Em demonstração o banco é recriado do zero a cada abertura. Sem isso a
/// segunda apresentação começa com o lixo da primeira na tela.
fn preparar_demonstracao(config: &Config) -> Result<(), ErroApp> {
    registro::info("modo demonstração: recriando a base fictícia");

    let banco = config.banco();
    for sufixo in ["", "-journal", "-wal", "-shm"] {
        let mut caminho = banco.clone().into_os_string();
        caminho.push(sufixo);
        let caminho = PathBuf::from(caminho);
        if caminho.exists() {
            std::fs::remove_file(&caminho).map_err(|e| {
                ErroApp::validacao(format!(
                    "não foi possível recriar a base de demonstração ({}): {e}",
                    caminho.display()
                ))
            })?;
        }
    }
    Ok(())
}
