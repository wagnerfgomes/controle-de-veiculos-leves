//! Cadastro de veículos.
//!
//! Com base nova e ninguém cadastrado, esta tela é a porta de entrada do
//! sistema: a primeira coisa que alguém faz é cadastrar a frota. Precisa ser
//! rápida de usar e difícil de sujar.

use rusqlite::{params, Connection};
use tauri::State;

use crate::db::auditoria::{self, Evento};
use crate::dominio::nome::frota_norm;
use crate::dominio::semelhanca;
use crate::dominio::validacao;
use crate::erro::ErroApp;
use crate::estado::Estado;
use crate::modelo::{ParDuplicata, Veiculo, VeiculoDisponibilidade};

const ENTIDADE: &str = "veiculos";

pub fn listar_com(conexao: &Connection, incluir_inativos: bool) -> Result<Vec<Veiculo>, ErroApp> {
    let sql = format!(
        "SELECT {} FROM veiculos {} ORDER BY frota",
        Veiculo::COLUNAS,
        if incluir_inativos {
            ""
        } else {
            "WHERE ativo = 1"
        }
    );
    let mut consulta = conexao.prepare(&sql)?;
    let linhas = consulta.query_map([], Veiculo::da_linha)?;
    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

pub fn por_id(conexao: &Connection, id: i64) -> Result<Veiculo, ErroApp> {
    let sql = format!("SELECT {} FROM veiculos WHERE id = ?1", Veiculo::COLUNAS);
    conexao
        .query_row(&sql, [id], Veiculo::da_linha)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                ErroApp::nao_encontrado("Veículo não encontrado.")
            }
            outro => ErroApp::from(outro),
        })
}

fn tem_viagem_aberta(conexao: &Connection, id: i64) -> Result<bool, ErroApp> {
    let total: i64 = conexao.query_row(
        "SELECT COUNT(*) FROM saidas
         WHERE veiculo_id = ?1 AND dt_chegada IS NULL AND excluida_em IS NULL",
        [id],
        |l| l.get(0),
    )?;
    Ok(total > 0)
}

fn semelhantes(conexao: &Connection, alvo: &str) -> Result<Vec<Veiculo>, ErroApp> {
    Ok(listar_com(conexao, true)?
        .into_iter()
        .filter(|v| semelhanca::frotas_semelhantes(&v.frota, alvo))
        .collect())
}

fn erro_semelhante(candidatos: &[Veiculo]) -> ErroApp {
    let frotas = candidatos
        .iter()
        .map(|v| v.frota.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let ids = candidatos
        .iter()
        .map(|v| v.id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    ErroApp::semelhante(format!("Já existe a frota {frotas}. É o mesmo veículo?"))
        .com_detalhe(format!("IDS:{ids}"))
}

// --------------------------------------------------------------- comandos ---

#[tauri::command]
pub fn listar_veiculos(
    estado: State<'_, Estado>,
    incluir_inativos: bool,
) -> Result<Vec<Veiculo>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    listar_com(&conexao, incluir_inativos)
}

/// O combo mostra os ocupados esmaecidos, com quem está e desde quando, em vez
/// de escondê-los: quem registra precisa saber a quem recorrer.
#[tauri::command]
pub fn listar_veiculos_disponiveis(
    estado: State<'_, Estado>,
) -> Result<Vec<VeiculoDisponibilidade>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;

    let sql = format!(
        "SELECT {}, m.nome, s.dt_saida, s.destino
         FROM veiculos v
         LEFT JOIN saidas s
                ON s.veiculo_id = v.id
               AND s.dt_chegada IS NULL
               AND s.excluida_em IS NULL
         LEFT JOIN motoristas m ON m.id = s.motorista_id
         WHERE v.ativo = 1
         ORDER BY v.frota",
        Veiculo::COLUNAS
            .split(", ")
            .map(|c| format!("v.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut consulta = conexao.prepare(&sql)?;
    let linhas = consulta.query_map([], |l| {
        let veiculo = Veiculo::da_linha(l)?;
        let motorista_atual: Option<String> = l.get(11)?;
        let desde: Option<String> = l.get(12)?;
        let destino_atual: Option<String> = l.get(13)?;
        Ok(VeiculoDisponibilidade {
            disponivel: desde.is_none(),
            veiculo,
            motorista_atual,
            desde,
            destino_atual,
        })
    })?;

    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn buscar_veiculos(estado: State<'_, Estado>, termo: String) -> Result<Vec<Veiculo>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    let alvo = frota_norm(&termo);
    if alvo.is_empty() {
        return listar_com(&conexao, false);
    }

    let sql = format!(
        "SELECT {} FROM veiculos
         WHERE ativo = 1
           AND (frota LIKE ?1 OR IFNULL(placa, '') LIKE ?1 OR UPPER(IFNULL(modelo, '')) LIKE ?1)
         ORDER BY frota",
        Veiculo::COLUNAS
    );
    let mut consulta = conexao.prepare(&sql)?;
    let linhas = consulta.query_map([format!("%{alvo}%")], Veiculo::da_linha)?;
    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn criar_veiculo(
    estado: State<'_, Estado>,
    frota: String,
    placa: Option<String>,
    modelo: Option<String>,
    marca: Option<String>,
    ano: Option<i64>,
    tipo: Option<String>,
    hodometro_atual: Option<i64>,
    observacao: Option<String>,
    confirmado: bool,
) -> Result<Veiculo, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    let norm = frota_norm(&frota);
    validacao::validar_frota(&norm)?;

    if !confirmado {
        let candidatos = semelhantes(&conexao, &norm)?;
        if !candidatos.is_empty() {
            return Err(erro_semelhante(&candidatos));
        }
    }

    let transacao = conexao.transaction()?;
    transacao.execute(
        "INSERT INTO veiculos
             (frota, placa, modelo, marca, ano, tipo, hodometro_atual, observacao)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            norm,
            limpar_maiusculo(placa),
            limpar(modelo),
            limpar(marca),
            ano,
            limpar(tipo),
            hodometro_atual,
            limpar(observacao)
        ],
    )?;
    let id = transacao.last_insert_rowid();

    let criado = por_id(&transacao, id)?;
    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::CRIAR)
            .id(id)
            .depois(&criado),
    )?;
    transacao.commit()?;

    Ok(criado)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn atualizar_veiculo(
    estado: State<'_, Estado>,
    id: i64,
    frota: String,
    placa: Option<String>,
    modelo: Option<String>,
    marca: Option<String>,
    ano: Option<i64>,
    tipo: Option<String>,
    hodometro_atual: Option<i64>,
    observacao: Option<String>,
    ativo: bool,
) -> Result<Veiculo, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    let norm = frota_norm(&frota);
    validacao::validar_frota(&norm)?;

    let transacao = conexao.transaction()?;
    let antes = por_id(&transacao, id)?;

    transacao.execute(
        "UPDATE veiculos SET frota = ?1, placa = ?2, modelo = ?3, marca = ?4, ano = ?5,
                             tipo = ?6, hodometro_atual = ?7, observacao = ?8, ativo = ?9
         WHERE id = ?10",
        params![
            norm,
            limpar_maiusculo(placa),
            limpar(modelo),
            limpar(marca),
            ano,
            limpar(tipo),
            hodometro_atual,
            limpar(observacao),
            i64::from(ativo),
            id
        ],
    )?;

    let depois = por_id(&transacao, id)?;
    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::ATUALIZAR)
            .id(id)
            .antes(&antes)
            .depois(&depois),
    )?;
    transacao.commit()?;

    Ok(depois)
}

#[tauri::command]
pub fn inativar_veiculo(estado: State<'_, Estado>, id: i64) -> Result<(), ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    let transacao = conexao.transaction()?;
    let antes = por_id(&transacao, id)?;

    if tem_viagem_aberta(&transacao, id)? {
        return Err(ErroApp::validacao(
            "Este veículo está em viagem. Encerre a viagem antes de inativar.",
        ));
    }

    transacao.execute("UPDATE veiculos SET ativo = 0 WHERE id = ?1", [id])?;
    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::INATIVAR)
            .id(id)
            .antes(&antes),
    )?;
    transacao.commit()?;

    Ok(())
}

#[tauri::command]
pub fn fundir_veiculos(
    estado: State<'_, Estado>,
    manter_id: i64,
    remover_id: i64,
) -> Result<u32, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    let transacao = conexao.transaction()?;
    let manter = por_id(&transacao, manter_id)?;
    let remover = por_id(&transacao, remover_id)?;

    validacao::validar_fusao(
        manter_id,
        remover_id,
        tem_viagem_aberta(&transacao, manter_id)?,
        tem_viagem_aberta(&transacao, remover_id)?,
    )?;

    let reapontadas = transacao.execute(
        "UPDATE saidas SET veiculo_id = ?1, atualizado_em = ?2 WHERE veiculo_id = ?3",
        params![manter_id, crate::dominio::datahora::agora(), remover_id],
    )?;

    transacao.execute("DELETE FROM veiculos WHERE id = ?1", [remover_id])?;

    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::FUNDIR)
            .id(manter_id)
            .antes(&remover)
            .depois(serde_json::json!({
                "mantido": manter,
                "saidas_reapontadas": reapontadas,
            })),
    )?;
    transacao.commit()?;

    Ok(reapontadas as u32)
}

#[tauri::command]
pub fn possiveis_duplicatas_veiculos(
    estado: State<'_, Estado>,
) -> Result<Vec<ParDuplicata<Veiculo>>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    let todos = listar_com(&conexao, true)?;

    let mut pares = Vec::new();
    for (i, a) in todos.iter().enumerate() {
        for b in todos.iter().skip(i + 1) {
            if semelhanca::frotas_semelhantes(&a.frota, &b.frota) {
                pares.push(ParDuplicata {
                    a: a.clone(),
                    b: b.clone(),
                    distancia: semelhanca::distancia(&a.frota, &b.frota) as u32,
                });
            }
        }
    }
    Ok(pares)
}

fn limpar(valor: Option<String>) -> Option<String> {
    valor
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn limpar_maiusculo(valor: Option<String>) -> Option<String> {
    limpar(valor).map(|v| v.to_uppercase())
}
