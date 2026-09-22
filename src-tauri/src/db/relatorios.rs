//! As consultas da seção 7 do SPEC.
//!
//! Duas regras mudam os números e não podem ser "melhoradas": viagem aberta
//! **entra** na contagem de viagens e **não entra** na soma de horas, e `:ate` é
//! exclusivo. Um relatório que somasse horas de viagem sem chegada estaria
//! inventando dado.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::dominio::datahora;
use crate::erro::ErroApp;
use crate::identidade;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoRelatorio {
    UsoVeiculo,
    UsoMotorista,
}

impl TipoRelatorio {
    pub fn titulo(&self) -> &'static str {
        match self {
            TipoRelatorio::UsoVeiculo => "Uso de veículo",
            TipoRelatorio::UsoMotorista => "Uso por condutor",
        }
    }

    pub fn arquivo(&self) -> &'static str {
        match self {
            TipoRelatorio::UsoVeiculo => "uso-veiculo",
            TipoRelatorio::UsoMotorista => "uso-motorista",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Formato {
    Html,
    Pdf,
    Csv,
}

#[derive(Debug, Clone, Serialize)]
pub struct CabecalhoRelatorio {
    pub de: String,
    pub ate: String,
    pub viagens: u32,
    pub sem_chegada: u32,
    pub sem_chegada_pct: f64,
    pub chegadas_manuais: u32,
    pub chegadas_manuais_pct: f64,
    pub hodometro_completo: u32,
    pub hodometro_completo_pct: f64,
    pub emitido_em: String,
    pub emitido_por: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinhaRelatorio {
    pub rotulo: String,
    pub viagens: u32,
    pub abertas: u32,
    pub horas_totais: f64,
    /// `None` quando nenhuma viagem da linha fechou.
    pub horas_media: Option<f64>,
    pub chegadas_manuais: u32,
    pub km: Option<i64>,
    pub veiculos_distintos: Option<u32>,
    pub frotas: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelatorioDados {
    pub tipo: TipoRelatorio,
    pub cabecalho: CabecalhoRelatorio,
    pub linhas: Vec<LinhaRelatorio>,
}

/// Sem estes números, o total de horas afirma algo que não sabe.
pub fn cabecalho(conexao: &Connection, de: &str, ate: &str) -> Result<CabecalhoRelatorio, ErroApp> {
    datahora::analisar_data(de)?;
    datahora::analisar_data(ate)?;

    let (viagens, sem_chegada, manuais, hodometro): (i64, i64, i64, i64) = conexao.query_row(
        "SELECT COUNT(*),
                SUM(CASE WHEN dt_chegada IS NULL THEN 1 ELSE 0 END),
                SUM(CASE WHEN chegada_manual = 1 THEN 1 ELSE 0 END),
                SUM(CASE WHEN hodometro_saida IS NOT NULL AND hodometro_chegada IS NOT NULL
                         THEN 1 ELSE 0 END)
           FROM saidas
          WHERE excluida_em IS NULL AND dt_saida >= ?1 AND dt_saida < ?2",
        [de, ate],
        |l| {
            Ok((
                l.get(0)?,
                l.get::<_, Option<i64>>(1)?.unwrap_or(0),
                l.get::<_, Option<i64>>(2)?.unwrap_or(0),
                l.get::<_, Option<i64>>(3)?.unwrap_or(0),
            ))
        },
    )?;

    let pct = |parte: i64| {
        if viagens == 0 {
            0.0
        } else {
            ((parte as f64 / viagens as f64) * 1000.0).round() / 10.0
        }
    };

    Ok(CabecalhoRelatorio {
        de: de.to_string(),
        ate: ate.to_string(),
        viagens: viagens as u32,
        sem_chegada: sem_chegada as u32,
        sem_chegada_pct: pct(sem_chegada),
        chegadas_manuais: manuais as u32,
        chegadas_manuais_pct: pct(manuais),
        hodometro_completo: hodometro as u32,
        hodometro_completo_pct: pct(hodometro),
        emitido_em: datahora::agora(),
        emitido_por: identidade::usuario(),
    })
}

pub fn uso_veiculo(
    conexao: &Connection,
    de: &str,
    ate: &str,
) -> Result<Vec<LinhaRelatorio>, ErroApp> {
    let mut consulta = conexao.prepare(
        "SELECT v.frota, v.modelo,
                COUNT(*)                                      AS viagens,
                SUM(CASE WHEN s.dt_chegada IS NULL THEN 1 ELSE 0 END) AS abertas,
                ROUND(SUM(CASE WHEN s.dt_chegada IS NOT NULL
                          THEN (julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24
                          ELSE 0 END), 2)                     AS horas_totais,
                ROUND(AVG(CASE WHEN s.dt_chegada IS NOT NULL
                          THEN (julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24
                          END), 2)                            AS horas_media,
                SUM(CASE WHEN s.chegada_manual = 1 THEN 1 ELSE 0 END) AS chegadas_manuais,
                SUM(CASE WHEN s.hodometro_chegada IS NOT NULL
                          AND s.hodometro_saida IS NOT NULL
                         THEN s.hodometro_chegada - s.hodometro_saida ELSE 0 END) AS km
           FROM saidas s
           JOIN veiculos v ON v.id = s.veiculo_id
          WHERE s.excluida_em IS NULL
            AND s.dt_saida >= ?1 AND s.dt_saida < ?2
          GROUP BY v.id
          ORDER BY horas_totais DESC",
    )?;

    let linhas = consulta.query_map([de, ate], |l| {
        let frota: String = l.get(0)?;
        let modelo: Option<String> = l.get(1)?;
        Ok(LinhaRelatorio {
            rotulo: match modelo {
                Some(m) if !m.is_empty() => format!("{frota} — {m}"),
                _ => frota,
            },
            viagens: l.get::<_, i64>(2)? as u32,
            abertas: l.get::<_, Option<i64>>(3)?.unwrap_or(0) as u32,
            horas_totais: l.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
            horas_media: l.get::<_, Option<f64>>(5)?,
            chegadas_manuais: l.get::<_, Option<i64>>(6)?.unwrap_or(0) as u32,
            km: l.get::<_, Option<i64>>(7)?,
            veiculos_distintos: None,
            frotas: None,
        })
    })?;

    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

pub fn uso_motorista(
    conexao: &Connection,
    de: &str,
    ate: &str,
) -> Result<Vec<LinhaRelatorio>, ErroApp> {
    let mut consulta = conexao.prepare(
        "SELECT m.nome, m.matricula,
                COUNT(*)                          AS viagens,
                COUNT(DISTINCT s.veiculo_id)      AS veiculos_distintos,
                GROUP_CONCAT(DISTINCT v.frota)    AS frotas,
                SUM(CASE WHEN s.dt_chegada IS NULL THEN 1 ELSE 0 END) AS abertas,
                ROUND(SUM(CASE WHEN s.dt_chegada IS NOT NULL
                          THEN (julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24
                          ELSE 0 END), 2)         AS horas_totais,
                ROUND(AVG(CASE WHEN s.dt_chegada IS NOT NULL
                          THEN (julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24
                          END), 2)                AS horas_media,
                SUM(CASE WHEN s.chegada_manual = 1 THEN 1 ELSE 0 END) AS chegadas_manuais
           FROM saidas s
           JOIN motoristas m ON m.id = s.motorista_id
           JOIN veiculos  v ON v.id = s.veiculo_id
          WHERE s.excluida_em IS NULL
            AND s.dt_saida >= ?1 AND s.dt_saida < ?2
          GROUP BY m.id
          ORDER BY horas_totais DESC",
    )?;

    let linhas = consulta.query_map([de, ate], |l| {
        let nome: String = l.get(0)?;
        let matricula: Option<String> = l.get(1)?;
        Ok(LinhaRelatorio {
            rotulo: match matricula {
                Some(m) if !m.is_empty() => format!("{nome} — {m}"),
                _ => nome,
            },
            viagens: l.get::<_, i64>(2)? as u32,
            veiculos_distintos: Some(l.get::<_, i64>(3)? as u32),
            // GROUP_CONCAT(DISTINCT ...) não aceita separador customizado no
            // SQLite; o padrão é vírgula, e a formatação fica no Rust.
            frotas: l
                .get::<_, Option<String>>(4)?
                .map(|f| f.split(',').collect::<Vec<_>>().join(", ")),
            abertas: l.get::<_, Option<i64>>(5)?.unwrap_or(0) as u32,
            horas_totais: l.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
            horas_media: l.get::<_, Option<f64>>(7)?,
            chegadas_manuais: l.get::<_, Option<i64>>(8)?.unwrap_or(0) as u32,
            km: None,
        })
    })?;

    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

pub fn montar(
    conexao: &Connection,
    tipo: TipoRelatorio,
    de: &str,
    ate: &str,
) -> Result<RelatorioDados, ErroApp> {
    let cabecalho = cabecalho(conexao, de, ate)?;
    let linhas = match tipo {
        TipoRelatorio::UsoVeiculo => uso_veiculo(conexao, de, ate)?,
        TipoRelatorio::UsoMotorista => uso_motorista(conexao, de, ate)?,
    };
    Ok(RelatorioDados {
        tipo,
        cabecalho,
        linhas,
    })
}

/// O detalhamento de quem pegou o carro, que cada linha de uso de veículo abre.
pub fn detalhe_veiculo(
    conexao: &Connection,
    veiculo_id: i64,
    de: &str,
    ate: &str,
) -> Result<Vec<DetalheViagem>, ErroApp> {
    let mut consulta = conexao.prepare(
        "SELECT m.nome, s.dt_saida, s.dt_chegada, s.turno, s.destino, s.atividade,
                s.chegada_manual,
                CASE WHEN s.dt_chegada IS NULL THEN NULL
                     ELSE ROUND((julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24, 2)
                END AS horas
           FROM saidas s
           JOIN motoristas m ON m.id = s.motorista_id
          WHERE s.veiculo_id = ?1
            AND s.excluida_em IS NULL
            AND s.dt_saida >= ?2 AND s.dt_saida < ?3
          ORDER BY s.dt_saida",
    )?;

    let linhas = consulta.query_map(rusqlite::params![veiculo_id, de, ate], |l| {
        Ok(DetalheViagem {
            motorista: l.get(0)?,
            dt_saida: l.get(1)?,
            dt_chegada: l.get(2)?,
            turno: l.get(3)?,
            destino: l.get(4)?,
            atividade: l.get(5)?,
            chegada_manual: l.get::<_, i64>(6)? != 0,
            horas: l.get(7)?,
        })
    })?;

    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct DetalheViagem {
    pub motorista: String,
    pub dt_saida: String,
    pub dt_chegada: Option<String>,
    pub turno: String,
    pub destino: Option<String>,
    pub atividade: String,
    pub chegada_manual: bool,
    pub horas: Option<f64>,
}
