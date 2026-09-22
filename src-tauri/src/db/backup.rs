//! Backup por `VACUUM INTO` e a rotação que impede a pasta de crescer sem fim.
//!
//! `VACUUM INTO` produz uma cópia consistente por construção. Cópia byte a byte
//! de um arquivo que está sendo escrito pode capturar páginas de dois estados, e
//! o resultado parece um backup até o dia em que alguém tenta restaurá-lo.

use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDateTime};
use rusqlite::Connection;

use crate::dominio::datahora;
use crate::erro::ErroApp;
use crate::registro;

const PREFIXO: &str = "controle_";
const EXTENSAO: &str = "db";

pub fn gerar(conexao: &Connection, pasta_backups: &Path) -> Result<PathBuf, ErroApp> {
    std::fs::create_dir_all(pasta_backups).map_err(|e| {
        ErroApp::conexao_perdida().com_detalhe(format!(
            "não foi possível acessar {}: {e}",
            pasta_backups.display()
        ))
    })?;

    let destino = pasta_backups.join(format!(
        "{PREFIXO}{}.{EXTENSAO}",
        datahora::agora_para_arquivo()
    ));

    // Um backup por minuto basta: o nome só tem precisão de minuto, e
    // sobrescrever faria o VACUUM INTO falhar com "output file already exists".
    if destino.exists() {
        return Ok(destino);
    }

    let caminho = destino.to_string_lossy().replace('\'', "''");
    conexao.execute_batch(&format!("VACUUM INTO '{caminho}'"))?;
    registro::info(&format!("backup gerado em {}", destino.display()));

    Ok(destino)
}

/// | Faixa | Mantém |
/// | Últimas 48 h | todos |
/// | Últimos 30 dias | o último de cada dia |
/// | Últimos 12 meses | o último de cada mês |
/// | Anteriores | descarta |
pub fn rotacionar(pasta_backups: &Path) -> Result<Vec<PathBuf>, ErroApp> {
    let agora = chrono::Local::now().naive_local();
    let mut arquivos = listar(pasta_backups)?;
    arquivos.sort_by(|a, b| a.0.cmp(&b.0));

    let manter = selecionar_para_manter(&arquivos, agora);

    let mut removidos = Vec::new();
    for (_, caminho) in &arquivos {
        if manter.contains(caminho) {
            continue;
        }
        match std::fs::remove_file(caminho) {
            Ok(()) => removidos.push(caminho.clone()),
            Err(e) => registro::aviso(&format!(
                "não foi possível remover backup {}: {e}",
                caminho.display()
            )),
        }
    }

    Ok(removidos)
}

/// Separada da remoção para poder ser testada sem tocar em disco de verdade.
fn selecionar_para_manter(
    arquivos: &[(NaiveDateTime, PathBuf)],
    agora: NaiveDateTime,
) -> Vec<PathBuf> {
    use std::collections::HashMap;

    let limite_48h = agora - chrono::Duration::hours(48);
    let limite_30d = agora - chrono::Duration::days(30);
    let limite_12m = agora - chrono::Duration::days(365);

    let mut ultimo_do_dia: HashMap<(i32, u32, u32), (NaiveDateTime, PathBuf)> = HashMap::new();
    let mut ultimo_do_mes: HashMap<(i32, u32), (NaiveDateTime, PathBuf)> = HashMap::new();
    let mut manter = Vec::new();

    for (quando, caminho) in arquivos {
        if *quando >= limite_48h {
            manter.push(caminho.clone());
            continue;
        }
        if *quando >= limite_30d {
            let chave = (quando.year(), quando.month(), quando.day());
            let atual = ultimo_do_dia.get(&chave);
            if atual.is_none_or(|(q, _)| quando > q) {
                ultimo_do_dia.insert(chave, (*quando, caminho.clone()));
            }
            continue;
        }
        if *quando >= limite_12m {
            let chave = (quando.year(), quando.month());
            let atual = ultimo_do_mes.get(&chave);
            if atual.is_none_or(|(q, _)| quando > q) {
                ultimo_do_mes.insert(chave, (*quando, caminho.clone()));
            }
        }
        // Mais velho que 12 meses: descarta.
    }

    manter.extend(ultimo_do_dia.into_values().map(|(_, c)| c));
    manter.extend(ultimo_do_mes.into_values().map(|(_, c)| c));
    manter
}

fn listar(pasta_backups: &Path) -> Result<Vec<(NaiveDateTime, PathBuf)>, ErroApp> {
    if !pasta_backups.exists() {
        return Ok(Vec::new());
    }

    let entradas = std::fs::read_dir(pasta_backups).map_err(|e| {
        ErroApp::conexao_perdida().com_detalhe(format!(
            "não foi possível listar {}: {e}",
            pasta_backups.display()
        ))
    })?;

    let mut encontrados = Vec::new();
    for entrada in entradas.flatten() {
        let caminho = entrada.path();
        if !caminho.is_file() {
            continue;
        }
        let Some(nome) = caminho.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if let Some(quando) = data_do_nome(nome) {
            encontrados.push((quando, caminho));
        }
    }

    Ok(encontrados)
}

/// `controle_2026-09-19_1430.db` → `2026-09-19 14:30`. Arquivo que não case com
/// o padrão fica de fora da rotação: nada que este módulo não criou é apagado.
fn data_do_nome(nome: &str) -> Option<NaiveDateTime> {
    let miolo = nome
        .strip_prefix(PREFIXO)?
        .strip_suffix(&format!(".{EXTENSAO}"))?;
    let (data, hora) = miolo.split_once('_')?;
    if hora.len() != 4 {
        return None;
    }
    datahora::analisar(&format!("{data} {}:{}", &hora[0..2], &hora[2..4])).ok()
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::db::conexao::testes::tempdir;

    fn sintetico(pasta: &Path, quando: NaiveDateTime) -> PathBuf {
        let nome = format!("{PREFIXO}{}.{EXTENSAO}", quando.format("%Y-%m-%d_%H%M"));
        let caminho = pasta.join(nome);
        std::fs::write(&caminho, b"backup").expect("escreve arquivo sintético");
        caminho
    }

    #[test]
    fn rotacao_com_quarenta_arquivos_sinteticos_mantem_a_politica() {
        let pasta = tempdir();
        let agora = datahora::analisar("2026-09-20 12:00").expect("data válida");

        let mut esperado_48h = 0;
        let mut arquivos = Vec::new();

        // 12 backups dentro de 48 h, de 4 em 4 horas.
        for i in 0..12 {
            let quando = agora - chrono::Duration::hours(i * 4);
            arquivos.push((quando, sintetico(&pasta, quando)));
            esperado_48h += 1;
        }
        // 14 backups entre 3 e 9 dias atrás, dois por dia.
        for dia in 3..10 {
            for hora in [9, 17] {
                let quando = (agora - chrono::Duration::days(dia))
                    .date()
                    .and_hms_opt(hora, 0, 0)
                    .expect("hora válida");
                arquivos.push((quando, sintetico(&pasta, quando)));
            }
        }
        // 8 backups entre 2 e 5 meses atrás, dois por mês.
        for mes in 2..6 {
            for dia in [3, 20] {
                let quando = (agora - chrono::Duration::days(mes * 30 + dia))
                    .date()
                    .and_hms_opt(8, 0, 0)
                    .expect("hora válida");
                arquivos.push((quando, sintetico(&pasta, quando)));
            }
        }
        // 6 backups com mais de 12 meses: todos devem sumir.
        for i in 0..6 {
            let quando = (agora - chrono::Duration::days(400 + i * 30))
                .date()
                .and_hms_opt(8, 0, 0)
                .expect("hora válida");
            arquivos.push((quando, sintetico(&pasta, quando)));
        }

        assert_eq!(arquivos.len(), 40);

        let manter = selecionar_para_manter(
            &{
                let mut v = arquivos.clone();
                v.sort_by(|a, b| a.0.cmp(&b.0));
                v
            },
            agora,
        );

        let dentro_48h = manter
            .iter()
            .filter(|c| {
                let nome = c.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                data_do_nome(nome)
                    .map(|q| q >= agora - chrono::Duration::hours(48))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(dentro_48h, esperado_48h, "tudo das últimas 48 h fica");

        // 7 dias com dois backups cada, dentro de 30 dias: sobra um por dia.
        // 4 meses com dois backups cada, dentro de 12 meses: sobra um por mês.
        assert_eq!(manter.len(), esperado_48h + 7 + 4);

        // Nada com mais de 12 meses sobrevive.
        for caminho in &manter {
            let nome = caminho
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let quando = data_do_nome(nome).expect("nome no padrão");
            assert!(quando >= agora - chrono::Duration::days(365));
        }

        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn arquivo_fora_do_padrao_nunca_e_removido() {
        let pasta = tempdir();
        let intruso = pasta.join("planilha-do-chefe.xlsx");
        std::fs::write(&intruso, b"nao apague").expect("escreve");

        let antigo = datahora::analisar("2020-01-01 08:00").expect("data válida");
        sintetico(&pasta, antigo);

        let removidos = rotacionar(&pasta).expect("rotaciona");

        assert!(intruso.exists(), "arquivo alheio foi removido");
        assert_eq!(removidos.len(), 1);

        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn vacuum_into_produz_copia_legivel() {
        let pasta = tempdir();
        let c = crate::db::conexao::abrir(&pasta.join("controle.db")).expect("abre");
        crate::db::migracoes::aplicar(&c, |_| Ok(())).expect("migra");

        let destino = gerar(&c, &pasta.join("backups")).expect("gera backup");
        assert!(destino.exists());

        let copia = crate::db::conexao::abrir(&destino).expect("abre a cópia");
        let nivel = crate::db::migracoes::nivel_atual(&copia).expect("lê nível");
        assert_eq!(nivel, 1, "a cópia precisa trazer o schema aplicado");

        let _ = std::fs::remove_dir_all(&pasta);
    }
}
