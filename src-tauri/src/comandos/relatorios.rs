//! Emissão manual de relatório.
//!
//! A varredura automática de meses pendentes é da Fase 9: só existe em produção
//! e não tinha como ser testada antes do veredito da rede.

use tauri::State;

use crate::db::relatorios::{self, Formato, RelatorioDados, TipoRelatorio};
use crate::dominio::relatorio;
use crate::erro::ErroApp;
use crate::estado::Estado;
use crate::modelo::FiltroSaidas;
use crate::registro;

/// Período efetivo do relatório. `ate` é **exclusivo**: para "setembro inteiro",
/// passar `2026-10-01`.
fn periodo(filtro: &FiltroSaidas) -> Result<(String, String), ErroApp> {
    let hoje = crate::dominio::datahora::hoje();

    let de = match &filtro.de {
        Some(d) => d.clone(),
        None => {
            let primeiro = hoje
                .with_day(1)
                .ok_or_else(|| ErroApp::validacao("não foi possível calcular o início do mês"))?;
            crate::dominio::datahora::formatar_data(primeiro)
        }
    };
    let ate = match &filtro.ate {
        Some(a) => a.clone(),
        None => crate::dominio::datahora::formatar_data(
            hoje.succ_opt()
                .ok_or_else(|| ErroApp::validacao("não foi possível calcular o fim do período"))?,
        ),
    };

    if de >= ate {
        return Err(ErroApp::validacao(
            "A data final precisa ser depois da inicial. O limite final é exclusivo.",
        ));
    }

    Ok((de, ate))
}

use chrono::Datelike;

#[tauri::command]
pub fn previa_relatorio(
    estado: State<'_, Estado>,
    tipo: TipoRelatorio,
    filtro: FiltroSaidas,
) -> Result<RelatorioDados, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    let (de, ate) = periodo(&filtro)?;
    relatorios::montar(&conexao, tipo, &de, &ate)
}

#[tauri::command]
pub fn gerar_relatorio(
    estado: State<'_, Estado>,
    tipo: TipoRelatorio,
    filtro: FiltroSaidas,
    formato: Formato,
) -> Result<String, ErroApp> {
    let sessao = estado.sessao()?;
    let (de, ate) = periodo(&filtro)?;

    let dados = {
        let conexao = sessao.travar_conexao()?;
        relatorios::montar(&conexao, tipo, &de, &ate)?
    };

    // Agrupado pelo mês de início do período, que é como a pasta da rede é
    // organizada: relatorios\AAAA-MM\.
    let pasta = sessao.config.relatorios().join(&de[0..7]);
    std::fs::create_dir_all(&pasta).map_err(|e| {
        ErroApp::conexao_perdida()
            .com_detalhe(format!("não foi possível criar {}: {e}", pasta.display()))
    })?;

    let (conteudo, extensao) = match formato {
        Formato::Csv => (relatorio::csv(&dados), "csv"),
        // PDF sai da impressão do próprio WebView sobre este HTML: gerar PDF em
        // Rust exigiria uma engine de layout inteira para um botão de imprimir.
        Formato::Html | Formato::Pdf => (relatorio::html(&dados), "html"),
    };

    let arquivo = pasta.join(format!("{}_{de}_a_{ate}.{extensao}", tipo.arquivo()));
    std::fs::write(&arquivo, conteudo.as_bytes()).map_err(|e| {
        ErroApp::conexao_perdida().com_detalhe(format!(
            "não foi possível gravar {}: {e}",
            arquivo.display()
        ))
    })?;

    registro::info(&format!("relatório gerado em {}", arquivo.display()));
    Ok(arquivo.display().to_string())
}

/// O detalhamento que cada linha de uso de veículo abre.
#[tauri::command]
pub fn detalhe_veiculo_relatorio(
    estado: State<'_, Estado>,
    veiculo_id: i64,
    filtro: FiltroSaidas,
) -> Result<Vec<relatorios::DetalheViagem>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    let (de, ate) = periodo(&filtro)?;
    relatorios::detalhe_veiculo(&conexao, veiculo_id, &de, &ate)
}
