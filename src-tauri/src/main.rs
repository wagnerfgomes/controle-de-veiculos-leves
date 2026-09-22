// Sem console atrás da janela no Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Um release de Linux que não seja a demo não existe: o lock em Linux é um stub
// de `flock`, que protege contra outra instância na mesma máquina e nada além
// disso. Deixá-lo passar para produção é como se descobre, meses depois, que
// nunca houve lock nenhum.
#[cfg(all(unix, not(debug_assertions), not(feature = "demo")))]
compile_error!(
    "Release em Linux só é permitido com --features demo. \
     O lock de produção é o handle exclusivo do Windows; em Linux ele é um stub."
);

mod comandos;
mod config;
mod db;
mod demo;
mod dominio;
mod erro;
mod estado;
mod identidade;
mod lock;
mod modelo;
mod registro;
mod sessao;

use std::sync::Arc;
use std::time::Duration;

use estado::Estado;
use lock::{Lock, INTERVALO_HEARTBEAT_SEGUNDOS};
use tauri::Manager;

fn main() {
    registro::info(&format!("iniciando — versão {}", env!("CARGO_PKG_VERSION")));

    let config = match config::carregar() {
        Ok(c) => c,
        Err(e) => {
            // Sem config não há o que abrir. Morrer aqui com o motivo no log é
            // melhor que abrir uma janela que não sabe onde estão os dados.
            registro::erro(&format!("configuração inválida: {e}"));
            eprintln!("Não foi possível ler a configuração: {e}");
            if let Some(log) = registro::caminho_log() {
                eprintln!("Detalhes em {log}");
            }
            std::process::exit(1);
        }
    };

    if !lock::lock_e_confiavel() {
        registro::aviso(
            "rodando fora do Windows: o lock é um stub e não protege contra outra máquina.",
        );
    }

    let estado = Estado::novo(config);

    tauri::Builder::default()
        .manage(estado)
        .setup(|app| {
            iniciar_heartbeat(app.handle().clone());
            Ok(())
        })
        .on_window_event(|janela, evento| {
            if matches!(evento, tauri::WindowEvent::Destroyed) {
                janela.state::<Estado>().fechar();
            }
        })
        .invoke_handler(tauri::generate_handler![
            comandos::sessao::iniciar_sessao,
            comandos::sessao::estado_sessao,
            comandos::sessao::info_bloqueio,
            comandos::sessao::tomar_posse_lock,
            comandos::sessao::forcar_backup,
            comandos::sessao::encerrar_sessao,
            comandos::motoristas::listar_motoristas,
            comandos::motoristas::buscar_motoristas,
            comandos::motoristas::criar_motorista,
            comandos::motoristas::atualizar_motorista,
            comandos::motoristas::inativar_motorista,
            comandos::motoristas::fundir_motoristas,
            comandos::motoristas::possiveis_duplicatas_motoristas,
            comandos::veiculos::listar_veiculos,
            comandos::veiculos::listar_veiculos_disponiveis,
            comandos::veiculos::buscar_veiculos,
            comandos::veiculos::criar_veiculo,
            comandos::veiculos::atualizar_veiculo,
            comandos::veiculos::inativar_veiculo,
            comandos::veiculos::fundir_veiculos,
            comandos::veiculos::possiveis_duplicatas_veiculos,
            comandos::saidas::abrir_saida,
            comandos::saidas::encerrar_saida,
            comandos::saidas::editar_saida,
            comandos::saidas::excluir_saida,
            comandos::saidas::listar_saidas_abertas,
            comandos::saidas::listar_saidas,
            comandos::saidas::kpis,
            comandos::saidas::destinos_recentes,
            comandos::relatorios::previa_relatorio,
            comandos::relatorios::gerar_relatorio,
            comandos::relatorios::detalhe_veiculo_relatorio,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            registro::erro(&format!("falha ao iniciar a janela: {e}"));
            std::process::exit(1);
        });
}

/// Reescreve `controle.lock.info` a cada 30 s. É o carimbo que a outra máquina
/// lê para decidir se a sessão morreu.
///
/// A queda para modo degradado depois de 3 falhas fica para a Fase 9, junto com
/// `reconectar()`: ligar o degradado sem a saída deixaria o operador preso numa
/// tela que recusa tudo e não oferece caminho de volta.
fn iniciar_heartbeat(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut relogio = tokio::time::interval(Duration::from_secs(INTERVALO_HEARTBEAT_SEGUNDOS));

        loop {
            relogio.tick().await;

            let estado = app.state::<Estado>();
            let Ok(sessao) = estado.sessao() else {
                continue;
            };

            bater(&sessao);
        }
    });
}

fn bater(sessao: &Arc<sessao::Sessao>) {
    let Ok(guarda) = sessao.lock.lock() else {
        return;
    };
    let Some(lock) = guarda.as_ref() else {
        return;
    };

    match lock.heartbeat() {
        Ok(()) => {
            if let Ok(mut falhas) = sessao.falhas_heartbeat.lock() {
                *falhas = 0;
            }
        }
        Err(e) => {
            let total = sessao
                .falhas_heartbeat
                .lock()
                .map(|mut f| {
                    *f += 1;
                    *f
                })
                .unwrap_or(0);
            registro::erro(&format!("heartbeat falhou ({total}ª vez): {e}"));

            if total >= lock::FALHAS_PARA_DEGRADAR {
                registro::erro(
                    "o heartbeat falhou três vezes seguidas: a rede provavelmente caiu e o \
                     lock pode ter sido perdido. A queda para modo degradado entra na Fase 9, \
                     junto com reconectar().",
                );
            }
        }
    }
}
