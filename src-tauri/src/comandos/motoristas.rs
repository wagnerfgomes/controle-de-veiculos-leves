//! Cadastro de condutores.
//!
//! O comando orquestra transação, auditoria e tradução de erro, e nada mais.
//! Regra de negócio vive em `dominio/`.

use rusqlite::{params, Connection};
use tauri::State;

use crate::db::auditoria::{self, Evento};
use crate::dominio::nome::nome_norm;
use crate::dominio::semelhanca;
use crate::dominio::validacao;
use crate::erro::ErroApp;
use crate::estado::Estado;
use crate::modelo::{Motorista, ParDuplicata};

const ENTIDADE: &str = "motoristas";

pub fn listar_com(conexao: &Connection, incluir_inativos: bool) -> Result<Vec<Motorista>, ErroApp> {
    let sql = format!(
        "SELECT {} FROM motoristas {} ORDER BY nome",
        Motorista::COLUNAS,
        if incluir_inativos {
            ""
        } else {
            "WHERE ativo = 1"
        }
    );
    let mut consulta = conexao.prepare(&sql)?;
    let linhas = consulta.query_map([], Motorista::da_linha)?;
    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

fn por_id(conexao: &Connection, id: i64) -> Result<Motorista, ErroApp> {
    let sql = format!(
        "SELECT {} FROM motoristas WHERE id = ?1",
        Motorista::COLUNAS
    );
    conexao
        .query_row(&sql, [id], Motorista::da_linha)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                ErroApp::nao_encontrado("Condutor não encontrado.")
            }
            outro => ErroApp::from(outro),
        })
}

fn tem_viagem_aberta(conexao: &Connection, id: i64) -> Result<bool, ErroApp> {
    let total: i64 = conexao.query_row(
        "SELECT COUNT(*) FROM saidas
         WHERE motorista_id = ?1 AND dt_chegada IS NULL AND excluida_em IS NULL",
        [id],
        |l| l.get(0),
    )?;
    Ok(total > 0)
}

/// Candidatos a duplicata do nome informado. Roda em memória: a base tem
/// centenas de linhas, não milhões, e Levenshtein em SQL não existe.
fn semelhantes(conexao: &Connection, alvo_norm: &str) -> Result<Vec<Motorista>, ErroApp> {
    Ok(listar_com(conexao, true)?
        .into_iter()
        .filter(|m| semelhanca::nomes_semelhantes(&m.nome_norm, alvo_norm))
        .collect())
}

fn erro_semelhante(candidatos: &[Motorista]) -> ErroApp {
    let nomes = candidatos
        .iter()
        .map(|m| m.nome.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let ids = candidatos
        .iter()
        .map(|m| m.id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    ErroApp::semelhante(format!("Já existe {nomes}. É a mesma pessoa?"))
        .com_detalhe(format!("IDS:{ids}"))
}

// --------------------------------------------------------------- comandos ---

#[tauri::command]
pub fn listar_motoristas(
    estado: State<'_, Estado>,
    incluir_inativos: bool,
) -> Result<Vec<Motorista>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    listar_com(&conexao, incluir_inativos)
}

#[tauri::command]
pub fn buscar_motoristas(
    estado: State<'_, Estado>,
    termo: String,
) -> Result<Vec<Motorista>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    let alvo = nome_norm(&termo);
    if alvo.is_empty() {
        return listar_com(&conexao, false);
    }

    let sql = format!(
        "SELECT {} FROM motoristas
         WHERE ativo = 1 AND (nome_norm LIKE ?1 OR IFNULL(matricula, '') LIKE ?1)
         ORDER BY nome",
        Motorista::COLUNAS
    );
    let mut consulta = conexao.prepare(&sql)?;
    let linhas = consulta.query_map([format!("%{alvo}%")], Motorista::da_linha)?;
    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

/// `confirmado = false` procura parecido e devolve `SEMELHANTE` **sem gravar
/// nada**. Gravar e perguntar depois é exatamente o bug que produziu 182 nomes
/// para cerca de 70 pessoas no sistema antigo.
#[tauri::command]
pub fn criar_motorista(
    estado: State<'_, Estado>,
    nome: String,
    matricula: Option<String>,
    setor: Option<String>,
    confirmado: bool,
) -> Result<Motorista, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    validacao::validar_nome(&nome)?;
    let norm = nome_norm(&nome);

    if !confirmado {
        let candidatos = semelhantes(&conexao, &norm)?;
        if !candidatos.is_empty() {
            return Err(erro_semelhante(&candidatos));
        }
    }

    let transacao = conexao.transaction()?;
    let id = {
        transacao.execute(
            "INSERT INTO motoristas (nome, nome_norm, matricula, setor) VALUES (?1, ?2, ?3, ?4)",
            params![nome.trim(), norm, limpar(matricula), limpar(setor)],
        )?;
        transacao.last_insert_rowid()
    };

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
pub fn atualizar_motorista(
    estado: State<'_, Estado>,
    id: i64,
    nome: String,
    matricula: Option<String>,
    setor: Option<String>,
    ativo: bool,
) -> Result<Motorista, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    validacao::validar_nome(&nome)?;
    let norm = nome_norm(&nome);

    let transacao = conexao.transaction()?;
    let antes = por_id(&transacao, id)?;

    transacao.execute(
        "UPDATE motoristas SET nome = ?1, nome_norm = ?2, matricula = ?3, setor = ?4, ativo = ?5
         WHERE id = ?6",
        params![
            nome.trim(),
            norm,
            limpar(matricula),
            limpar(setor),
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
pub fn inativar_motorista(estado: State<'_, Estado>, id: i64) -> Result<(), ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    let transacao = conexao.transaction()?;
    let antes = por_id(&transacao, id)?;

    if tem_viagem_aberta(&transacao, id)? {
        return Err(ErroApp::validacao(
            "Este condutor está em viagem. Encerre a viagem antes de inativar.",
        ));
    }

    transacao.execute("UPDATE motoristas SET ativo = 0 WHERE id = ?1", [id])?;
    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::INATIVAR)
            .id(id)
            .antes(&antes),
    )?;
    transacao.commit()?;

    Ok(())
}

/// Reaponta as saídas, guarda o removido inteiro em JSON na auditoria e apaga o
/// duplicado. Devolve quantas saídas mudaram de dono, que é o número que a tela
/// mostra na confirmação.
#[tauri::command]
pub fn fundir_motoristas(
    estado: State<'_, Estado>,
    manter_id: i64,
    remover_id: i64,
) -> Result<u32, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;
    fundir_com(&mut conexao, manter_id, remover_id)
}

pub fn fundir_com(
    conexao: &mut Connection,
    manter_id: i64,
    remover_id: i64,
) -> Result<u32, ErroApp> {
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
        "UPDATE saidas SET motorista_id = ?1, atualizado_em = ?2 WHERE motorista_id = ?3",
        params![manter_id, crate::dominio::datahora::agora(), remover_id],
    )?;

    transacao.execute("DELETE FROM motoristas WHERE id = ?1", [remover_id])?;

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

/// O indicador de duplicatas da tela de Gestão. É a rede de segurança do
/// cadastro rápido: transforma a limpeza em hábito, em vez de mutirão anual.
#[tauri::command]
pub fn possiveis_duplicatas_motoristas(
    estado: State<'_, Estado>,
) -> Result<Vec<ParDuplicata<Motorista>>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    let todos = listar_com(&conexao, true)?;

    let mut pares = Vec::new();
    for (i, a) in todos.iter().enumerate() {
        for b in todos.iter().skip(i + 1) {
            if semelhanca::nomes_semelhantes(&a.nome_norm, &b.nome_norm) {
                pares.push(ParDuplicata {
                    a: a.clone(),
                    b: b.clone(),
                    distancia: semelhanca::distancia(&a.nome_norm, &b.nome_norm) as u32,
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

#[cfg(test)]
mod testes {
    use super::*;
    use crate::comandos::saidas::{self, DadosAbertura};
    use crate::db::{conexao, migracoes};
    use crate::dominio::nome::frota_norm;

    fn base() -> Connection {
        let c = conexao::abrir_em_memoria().expect("abre");
        migracoes::aplicar(&c, |_| Ok(())).expect("migra");

        for frota in ["907015", "907018"] {
            c.execute(
                "INSERT INTO veiculos (frota) VALUES (?1)",
                [frota_norm(frota)],
            )
            .expect("cria veículo");
        }
        for nome in ["Angelo Junior", "Anjelo Junior"] {
            c.execute(
                "INSERT INTO motoristas (nome, nome_norm) VALUES (?1, ?2)",
                rusqlite::params![nome, nome_norm(nome)],
            )
            .expect("cria condutor");
        }
        c
    }

    fn dia(dias_atras: i64) -> String {
        crate::dominio::datahora::formatar_data(
            crate::dominio::datahora::hoje() - chrono::Duration::days(dias_atras),
        )
    }

    fn viagem(c: &mut Connection, veiculo_id: i64, motorista_id: i64, dias_atras: i64) {
        let dia = dia(dias_atras);
        let saida = saidas::abrir_com(
            c,
            DadosAbertura {
                veiculo_id,
                motorista_id,
                turno: "A".to_string(),
                destino: None,
                atividade: "Teste".to_string(),
                dt_saida: Some(format!("{dia} 08:00")),
                hodometro_saida: None,
                observacao: None,
            },
            false,
        )
        .expect("abre");
        saidas::encerrar_com(c, saida.id, Some(format!("{dia} 12:00")), None, false)
            .expect("encerra");
    }

    #[test]
    fn fundir_preserva_a_contagem_de_saidas() {
        let mut c = base();
        viagem(&mut c, 1, 1, 3);
        viagem(&mut c, 1, 1, 2);
        viagem(&mut c, 2, 2, 1);

        let antes: i64 = c
            .query_row("SELECT COUNT(*) FROM saidas", [], |l| l.get(0))
            .expect("conta");

        let reapontadas = fundir_com(&mut c, 1, 2).expect("funde");
        assert_eq!(reapontadas, 1);

        let depois: i64 = c
            .query_row("SELECT COUNT(*) FROM saidas", [], |l| l.get(0))
            .expect("conta");
        assert_eq!(antes, depois, "fusão não pode perder nem duplicar viagem");

        let do_sobrevivente: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM saidas WHERE motorista_id = 1",
                [],
                |l| l.get(0),
            )
            .expect("conta");
        assert_eq!(do_sobrevivente, 3, "todas as saídas ficam sob quem restou");

        let restantes: i64 = c
            .query_row("SELECT COUNT(*) FROM motoristas", [], |l| l.get(0))
            .expect("conta");
        assert_eq!(restantes, 1);
    }

    #[test]
    fn fundir_guarda_o_removido_inteiro_na_auditoria() {
        let mut c = base();
        fundir_com(&mut c, 1, 2).expect("funde");

        let antes: Option<String> = c
            .query_row(
                "SELECT antes FROM auditoria WHERE acao = ?1",
                [auditoria::FUNDIR],
                |l| l.get(0),
            )
            .expect("lê auditoria");

        let json = antes.expect("o removido precisa estar guardado");
        assert!(json.contains("Anjelo Junior"), "json: {json}");
    }

    #[test]
    fn fundir_com_viagem_aberta_e_recusado() {
        let mut c = base();
        saidas::abrir_com(
            &mut c,
            DadosAbertura {
                veiculo_id: 1,
                motorista_id: 2,
                turno: "A".to_string(),
                destino: None,
                atividade: "Teste".to_string(),
                dt_saida: Some(format!("{} 08:00", dia(1))),
                hodometro_saida: None,
                observacao: None,
            },
            false,
        )
        .expect("abre");

        let erro = fundir_com(&mut c, 1, 2).expect_err("deve recusar");
        assert_eq!(erro.codigo, "VALIDACAO");

        let restantes: i64 = c
            .query_row("SELECT COUNT(*) FROM motoristas", [], |l| l.get(0))
            .expect("conta");
        assert_eq!(restantes, 2, "nada pode ter sido apagado");
    }

    #[test]
    fn nome_repetido_apos_normalizar_e_recusado_pelo_unique() {
        let c = base();
        let erro = c
            .execute(
                "INSERT INTO motoristas (nome, nome_norm) VALUES (?1, ?2)",
                rusqlite::params!["ANGELO  JUNIOR", nome_norm("ANGELO  JUNIOR")],
            )
            .expect_err("o UNIQUE de nome_norm precisa barrar");

        assert_eq!(ErroApp::from(erro).codigo, "DUPLICADO");
    }

    #[test]
    fn possiveis_duplicatas_encontra_o_par_parecido() {
        let c = base();
        let todos = listar_com(&c, true).expect("lista");
        let pares: Vec<_> = todos
            .iter()
            .enumerate()
            .flat_map(|(i, a)| {
                todos
                    .iter()
                    .skip(i + 1)
                    .filter(|b| semelhanca::nomes_semelhantes(&a.nome_norm, &b.nome_norm))
                    .map(move |b| (a.nome.clone(), b.nome.clone()))
            })
            .collect();

        assert_eq!(pares.len(), 1);
        assert_eq!(pares[0].0, "Angelo Junior");
    }
}
