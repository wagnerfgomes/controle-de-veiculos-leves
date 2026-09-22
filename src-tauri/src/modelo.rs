//! Structs que atravessam o `invoke()`.
//!
//! Os nomes ficam em `snake_case` dos dois lados: o espelho em TypeScript é
//! literal, e tradução de nome entre camadas é erro que o compilador não pega.

use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::erro::ErroApp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Motorista {
    pub id: i64,
    pub nome: String,
    pub nome_norm: String,
    pub matricula: Option<String>,
    pub setor: Option<String>,
    pub ativo: bool,
    pub criado_em: String,
}

impl Motorista {
    pub const COLUNAS: &'static str = "id, nome, nome_norm, matricula, setor, ativo, criado_em";

    pub fn da_linha(l: &Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: l.get(0)?,
            nome: l.get(1)?,
            nome_norm: l.get(2)?,
            matricula: l.get(3)?,
            setor: l.get(4)?,
            ativo: l.get::<_, i64>(5)? != 0,
            criado_em: l.get(6)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Veiculo {
    pub id: i64,
    pub frota: String,
    pub placa: Option<String>,
    pub modelo: Option<String>,
    pub marca: Option<String>,
    pub ano: Option<i64>,
    pub tipo: Option<String>,
    pub hodometro_atual: Option<i64>,
    pub ativo: bool,
    pub observacao: Option<String>,
    pub criado_em: String,
}

impl Veiculo {
    pub const COLUNAS: &'static str =
        "id, frota, placa, modelo, marca, ano, tipo, hodometro_atual, ativo, observacao, criado_em";

    pub fn da_linha(l: &Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: l.get(0)?,
            frota: l.get(1)?,
            placa: l.get(2)?,
            modelo: l.get(3)?,
            marca: l.get(4)?,
            ano: l.get(5)?,
            tipo: l.get(6)?,
            hodometro_atual: l.get(7)?,
            ativo: l.get::<_, i64>(8)? != 0,
            observacao: l.get(9)?,
            criado_em: l.get(10)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saida {
    pub id: i64,
    pub veiculo_id: i64,
    pub motorista_id: i64,
    pub turno: String,
    pub destino: Option<String>,
    pub atividade: String,
    pub dt_saida: String,
    pub dt_chegada: Option<String>,
    pub hodometro_saida: Option<i64>,
    pub hodometro_chegada: Option<i64>,
    pub chegada_manual: bool,
    pub observacao: Option<String>,
    pub excluida_em: Option<String>,
    pub excluida_por: Option<String>,
    pub excluida_motivo: Option<String>,
    pub criado_em: String,
    pub atualizado_em: Option<String>,
    /// Vem do `JOIN`. A tela mostra frota e nome, nunca o id.
    pub frota: Option<String>,
    pub motorista: Option<String>,
}

impl Saida {
    pub const COLUNAS: &'static str = "s.id, s.veiculo_id, s.motorista_id, s.turno, s.destino, \
         s.atividade, s.dt_saida, s.dt_chegada, s.hodometro_saida, s.hodometro_chegada, \
         s.chegada_manual, s.observacao, s.excluida_em, s.excluida_por, s.excluida_motivo, \
         s.criado_em, s.atualizado_em, v.frota, m.nome";

    pub const JUNCOES: &'static str = "FROM saidas s JOIN veiculos v ON v.id = s.veiculo_id \
         JOIN motoristas m ON m.id = s.motorista_id";

    pub fn da_linha(l: &Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: l.get(0)?,
            veiculo_id: l.get(1)?,
            motorista_id: l.get(2)?,
            turno: l.get(3)?,
            destino: l.get(4)?,
            atividade: l.get(5)?,
            dt_saida: l.get(6)?,
            dt_chegada: l.get(7)?,
            hodometro_saida: l.get(8)?,
            hodometro_chegada: l.get(9)?,
            chegada_manual: l.get::<_, i64>(10)? != 0,
            observacao: l.get(11)?,
            excluida_em: l.get(12)?,
            excluida_por: l.get(13)?,
            excluida_motivo: l.get(14)?,
            criado_em: l.get(15)?,
            atualizado_em: l.get(16)?,
            frota: l.get(17)?,
            motorista: l.get(18)?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SaidaAberta {
    pub id: i64,
    pub frota: String,
    pub motorista: String,
    pub destino: Option<String>,
    pub atividade: String,
    pub dt_saida: String,
    pub turno: String,
    pub horas_decorridas: f64,
    /// Acima de 24 h. É a linha vermelha do Painel.
    pub alerta: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VeiculoDisponibilidade {
    pub veiculo: Veiculo,
    pub disponivel: bool,
    pub motorista_atual: Option<String>,
    pub desde: Option<String>,
    pub destino_atual: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FiltroSaidas {
    pub de: Option<String>,
    pub ate: Option<String>,
    pub veiculo_id: Option<i64>,
    pub motorista_id: Option<i64>,
    pub turnos: Vec<String>,
    pub somente_abertas: bool,
    pub incluir_excluidas: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Kpis {
    pub viagens: u32,
    pub abertas: u32,
    pub condutores_distintos: u32,
    pub frotas_distintas: u32,
    /// `None` quando nenhuma viagem fechou. Zero na tela afirmaria que os carros
    /// rodaram sem gastar tempo.
    pub duracao_media_horas: Option<f64>,
    pub por_turno: [u32; 3],
}

/// Par de candidatos a duplicata, para o indicador da tela de Gestão.
#[derive(Debug, Clone, Serialize)]
pub struct ParDuplicata<T> {
    pub a: T,
    pub b: T,
    pub distancia: u32,
}

/// Monta o `WHERE` compartilhado por listagem, KPIs e relatórios.
///
/// Uma função só porque "aberta" significa `dt_chegada IS NULL AND excluida_em
/// IS NULL`, e filtro incompleto ressuscita registro excluído ou esconde veículo
/// em uso.
pub fn clausulas_do_filtro(
    filtro: &FiltroSaidas,
) -> Result<(String, Vec<Box<dyn rusqlite::ToSql>>), ErroApp> {
    let mut clausulas: Vec<String> = Vec::new();
    let mut parametros: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if !filtro.incluir_excluidas {
        clausulas.push("s.excluida_em IS NULL".to_string());
    }
    if filtro.somente_abertas {
        clausulas.push("s.dt_chegada IS NULL AND s.excluida_em IS NULL".to_string());
    }

    if let Some(de) = &filtro.de {
        crate::dominio::datahora::analisar_data(de)?;
        clausulas.push(format!("s.dt_saida >= ?{}", parametros.len() + 1));
        parametros.push(Box::new(de.clone()));
    }
    if let Some(ate) = &filtro.ate {
        crate::dominio::datahora::analisar_data(ate)?;
        // `:ate` é exclusivo. Para "setembro inteiro", passar 2026-10-01.
        clausulas.push(format!("s.dt_saida < ?{}", parametros.len() + 1));
        parametros.push(Box::new(ate.clone()));
    }
    if let Some(id) = filtro.veiculo_id {
        clausulas.push(format!("s.veiculo_id = ?{}", parametros.len() + 1));
        parametros.push(Box::new(id));
    }
    if let Some(id) = filtro.motorista_id {
        clausulas.push(format!("s.motorista_id = ?{}", parametros.len() + 1));
        parametros.push(Box::new(id));
    }
    if !filtro.turnos.is_empty() {
        let mut marcadores = Vec::new();
        for turno in &filtro.turnos {
            crate::dominio::validacao::validar_turno(turno)?;
            marcadores.push(format!("?{}", parametros.len() + 1));
            parametros.push(Box::new(turno.clone()));
        }
        clausulas.push(format!("s.turno IN ({})", marcadores.join(", ")));
    }

    let onde = if clausulas.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clausulas.join(" AND "))
    };

    Ok((onde, parametros))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn filtro_vazio_ainda_esconde_excluidas() {
        let (onde, params) = clausulas_do_filtro(&FiltroSaidas::default()).expect("monta");
        assert_eq!(onde, "WHERE s.excluida_em IS NULL");
        assert!(params.is_empty());
    }

    #[test]
    fn somente_abertas_traz_as_duas_condicoes() {
        let filtro = FiltroSaidas {
            somente_abertas: true,
            ..Default::default()
        };
        let (onde, _) = clausulas_do_filtro(&filtro).expect("monta");
        assert!(onde.contains("s.dt_chegada IS NULL AND s.excluida_em IS NULL"));
    }

    #[test]
    fn periodo_usa_ate_exclusivo() {
        let filtro = FiltroSaidas {
            de: Some("2026-09-01".to_string()),
            ate: Some("2026-10-01".to_string()),
            ..Default::default()
        };
        let (onde, params) = clausulas_do_filtro(&filtro).expect("monta");
        assert!(onde.contains("s.dt_saida >= ?1"));
        assert!(
            onde.contains("s.dt_saida < ?2"),
            "o limite superior é exclusivo"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn data_invalida_no_filtro_e_recusada() {
        let filtro = FiltroSaidas {
            de: Some("01/09/2026".to_string()),
            ..Default::default()
        };
        assert!(clausulas_do_filtro(&filtro).is_err());
    }

    #[test]
    fn turno_invalido_no_filtro_e_recusado() {
        let filtro = FiltroSaidas {
            turnos: vec!["A".to_string(), "Z".to_string()],
            ..Default::default()
        };
        assert!(clausulas_do_filtro(&filtro).is_err());
    }
}
