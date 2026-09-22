//! Abertura, encerramento, edição e exclusão de viagem.
//!
//! "Aberta" significa `dt_chegada IS NULL AND excluida_em IS NULL`, sempre as
//! duas condições. Filtro incompleto ressuscita registro excluído ou esconde
//! veículo que está na rua.
//!
//! Cada comando é uma casca sobre uma função `*_com(&mut Connection, …)`. É o
//! que permite testar as duas regras de ouro sem subir o Tauri inteiro.

use rusqlite::{params, Connection};
use serde::Deserialize;
use tauri::State;

use crate::db::auditoria::{self, Evento};
use crate::dominio::{datahora, duracao, validacao};
use crate::erro::ErroApp;
use crate::estado::Estado;
use crate::identidade;
use crate::modelo::{clausulas_do_filtro, FiltroSaidas, Kpis, Saida, SaidaAberta};

const ENTIDADE: &str = "saidas";

pub fn por_id(conexao: &Connection, id: i64) -> Result<Saida, ErroApp> {
    let sql = format!(
        "SELECT {} {} WHERE s.id = ?1",
        Saida::COLUNAS,
        Saida::JUNCOES
    );
    conexao
        .query_row(&sql, [id], Saida::da_linha)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                ErroApp::nao_encontrado("Viagem não encontrada.")
            }
            outro => ErroApp::from(outro),
        })
}

/// O `SELECT` prévio existe para dar a mensagem boa ("está com o Fulano desde
/// as 07:40"), não para garantir a regra. Quem garante é o índice único parcial:
/// o `SELECT` não impede a corrida, o índice sim.
fn quem_esta_com_o_veiculo(
    conexao: &Connection,
    veiculo_id: i64,
) -> Result<Option<String>, ErroApp> {
    let mut consulta = conexao.prepare(
        "SELECT m.nome, s.dt_saida FROM saidas s
         JOIN motoristas m ON m.id = s.motorista_id
         WHERE s.veiculo_id = ?1 AND s.dt_chegada IS NULL AND s.excluida_em IS NULL",
    )?;
    let mut linhas = consulta.query([veiculo_id])?;
    let Some(linha) = linhas.next()? else {
        return Ok(None);
    };
    let nome: String = linha.get(0)?;
    let dt_saida: String = linha.get(1)?;
    Ok(Some(format!("{nome} · desde {dt_saida}")))
}

fn motorista_esta_em_viagem(conexao: &Connection, motorista_id: i64) -> Result<bool, ErroApp> {
    let total: i64 = conexao.query_row(
        "SELECT COUNT(*) FROM saidas
         WHERE motorista_id = ?1 AND dt_chegada IS NULL AND excluida_em IS NULL",
        [motorista_id],
        |l| l.get(0),
    )?;
    Ok(total > 0)
}

// ------------------------------------------------------------------ miolo ---

#[derive(Debug, Clone, Deserialize)]
pub struct DadosAbertura {
    pub veiculo_id: i64,
    pub motorista_id: i64,
    pub turno: String,
    pub destino: Option<String>,
    pub atividade: String,
    /// `None` = agora.
    pub dt_saida: Option<String>,
    pub hodometro_saida: Option<i64>,
    pub observacao: Option<String>,
}

pub fn abrir_com(
    conexao: &mut Connection,
    d: DadosAbertura,
    confirmado_avisos: bool,
) -> Result<Saida, ErroApp> {
    let saida_texto = d.dt_saida.clone().unwrap_or_else(datahora::agora);
    let momento = datahora::analisar(&saida_texto)?;

    let transacao = conexao.transaction()?;

    let veiculo = crate::comandos::veiculos::por_id(&transacao, d.veiculo_id)?;
    let motorista_ativo: bool = transacao
        .query_row(
            "SELECT ativo FROM motoristas WHERE id = ?1",
            [d.motorista_id],
            |l| Ok(l.get::<_, i64>(0)? != 0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                ErroApp::nao_encontrado("Condutor não encontrado.")
            }
            outro => ErroApp::from(outro),
        })?;

    let avisos = validacao::validar_abertura(&validacao::DadosAbertura {
        turno: d.turno.clone(),
        atividade: d.atividade.clone(),
        dt_saida: momento,
        hodometro_saida: d.hodometro_saida,
        veiculo_ativo: veiculo.ativo,
        motorista_ativo,
        hodometro_atual: veiculo.hodometro_atual,
    })?;

    if !avisos.is_empty() && !confirmado_avisos {
        return Err(validacao::erro_de_confirmacao(&avisos));
    }

    if let Some(com_quem) = quem_esta_com_o_veiculo(&transacao, d.veiculo_id)? {
        return Err(ErroApp::veiculo_em_uso().com_detalhe(format!("COM:{com_quem}")));
    }
    if motorista_esta_em_viagem(&transacao, d.motorista_id)? {
        return Err(ErroApp::motorista_em_uso());
    }

    transacao.execute(
        "INSERT INTO saidas
             (veiculo_id, motorista_id, turno, destino, atividade, dt_saida,
              hodometro_saida, observacao)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            d.veiculo_id,
            d.motorista_id,
            d.turno,
            limpar(d.destino),
            d.atividade.trim(),
            saida_texto,
            d.hodometro_saida,
            limpar(d.observacao)
        ],
    )?;
    let id = transacao.last_insert_rowid();

    let criada = por_id(&transacao, id)?;
    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::ABRIR_SAIDA)
            .id(id)
            .depois(&criada),
    )?;
    transacao.commit()?;

    Ok(criada)
}

pub fn encerrar_com(
    conexao: &mut Connection,
    id: i64,
    dt_chegada: Option<String>,
    hodometro_chegada: Option<i64>,
    confirmado_avisos: bool,
) -> Result<Saida, ErroApp> {
    // Chegada informada à mão vira `chegada_manual = 1`, e o relatório mostra o
    // percentual: é o indicador de quanto o registro está sendo feito depois.
    let manual = dt_chegada.is_some();
    let chegada_texto = dt_chegada.unwrap_or_else(datahora::agora);
    let chegada = datahora::analisar(&chegada_texto)?;

    let transacao = conexao.transaction()?;
    let antes = por_id(&transacao, id)?;

    if antes.excluida_em.is_some() {
        return Err(ErroApp::validacao("Esta viagem foi excluída."));
    }

    let avisos = validacao::validar_encerramento(&validacao::DadosEncerramento {
        dt_saida: datahora::analisar(&antes.dt_saida)?,
        dt_chegada: chegada,
        ja_encerrada: antes.dt_chegada.is_some(),
        hodometro_saida: antes.hodometro_saida,
        hodometro_chegada,
    })?;

    if !avisos.is_empty() && !confirmado_avisos {
        return Err(validacao::erro_de_confirmacao(&avisos));
    }

    transacao.execute(
        "UPDATE saidas
            SET dt_chegada = ?1, hodometro_chegada = ?2, chegada_manual = ?3, atualizado_em = ?4
          WHERE id = ?5",
        params![
            chegada_texto,
            hodometro_chegada,
            i64::from(manual),
            datahora::agora(),
            id
        ],
    )?;

    // Serve só para pré-preencher a próxima saída, e por isso nunca anda para
    // trás. Relatório nenhum lê este campo: ele pode estar defasado.
    if let Some(hodometro) = hodometro_chegada {
        transacao.execute(
            "UPDATE veiculos SET hodometro_atual = ?1
              WHERE id = ?2 AND (hodometro_atual IS NULL OR hodometro_atual < ?1)",
            params![hodometro, antes.veiculo_id],
        )?;
    }

    let depois = por_id(&transacao, id)?;
    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::ENCERRAR_SAIDA)
            .id(id)
            .antes(&antes)
            .depois(&depois),
    )?;
    transacao.commit()?;

    Ok(depois)
}

#[derive(Debug, Clone, Deserialize)]
pub struct DadosEdicao {
    pub id: i64,
    pub turno: String,
    pub destino: Option<String>,
    pub atividade: String,
    pub dt_saida: String,
    pub dt_chegada: Option<String>,
    pub hodometro_saida: Option<i64>,
    pub hodometro_chegada: Option<i64>,
    pub observacao: Option<String>,
}

pub fn editar_com(
    conexao: &mut Connection,
    d: DadosEdicao,
    confirmado_avisos: bool,
) -> Result<Saida, ErroApp> {
    let momento_saida = datahora::analisar(&d.dt_saida)?;
    validacao::validar_turno(&d.turno)?;
    validacao::validar_atividade(&d.atividade)?;

    if datahora::no_futuro_alem_de(momento_saida, validacao::FOLGA_FUTURO_MINUTOS) {
        return Err(
            ErroApp::validacao("A saída não pode ficar no futuro.").com_detalhe("CAMPO:dt_saida")
        );
    }

    let transacao = conexao.transaction()?;
    let antes = por_id(&transacao, d.id)?;

    if antes.excluida_em.is_some() {
        return Err(ErroApp::validacao("Esta viagem foi excluída."));
    }

    let mut avisos = Vec::new();
    if let Some(texto_chegada) = &d.dt_chegada {
        let chegada = datahora::analisar(texto_chegada)?;
        avisos = validacao::validar_encerramento(&validacao::DadosEncerramento {
            dt_saida: momento_saida,
            dt_chegada: chegada,
            ja_encerrada: false,
            hodometro_saida: d.hodometro_saida,
            hodometro_chegada: d.hodometro_chegada,
        })?;
    }

    if !avisos.is_empty() && !confirmado_avisos {
        return Err(validacao::erro_de_confirmacao(&avisos));
    }

    transacao.execute(
        "UPDATE saidas
            SET turno = ?1, destino = ?2, atividade = ?3, dt_saida = ?4, dt_chegada = ?5,
                hodometro_saida = ?6, hodometro_chegada = ?7, observacao = ?8,
                atualizado_em = ?9
          WHERE id = ?10",
        params![
            d.turno,
            limpar(d.destino),
            d.atividade.trim(),
            d.dt_saida,
            d.dt_chegada,
            d.hodometro_saida,
            d.hodometro_chegada,
            limpar(d.observacao),
            datahora::agora(),
            d.id
        ],
    )?;

    let depois = por_id(&transacao, d.id)?;
    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::EDITAR_SAIDA)
            .id(d.id)
            .antes(&antes)
            .depois(&depois),
    )?;
    transacao.commit()?;

    Ok(depois)
}

/// Exclusão é sempre lógica. Não existe `DELETE` de saída em nenhum caminho da
/// aplicação: o registro sai das telas e dos relatórios, libera o índice único,
/// e continua auditável.
pub fn excluir_com(conexao: &mut Connection, id: i64, motivo: &str) -> Result<(), ErroApp> {
    let transacao = conexao.transaction()?;
    let antes = por_id(&transacao, id)?;

    validacao::validar_exclusao(antes.excluida_em.is_some(), motivo)?;

    transacao.execute(
        "UPDATE saidas SET excluida_em = ?1, excluida_por = ?2, excluida_motivo = ?3,
                           atualizado_em = ?1
          WHERE id = ?4",
        params![datahora::agora(), identidade::usuario(), motivo.trim(), id],
    )?;

    auditoria::registrar(
        &transacao,
        Evento::novo(ENTIDADE, auditoria::EXCLUIR_SAIDA)
            .id(id)
            .antes(&antes)
            .depois(serde_json::json!({ "motivo": motivo.trim() })),
    )?;
    transacao.commit()?;

    Ok(())
}

pub fn abertas_com(conexao: &Connection) -> Result<Vec<SaidaAberta>, ErroApp> {
    let mut consulta = conexao.prepare(
        "SELECT s.id, v.frota, m.nome, s.destino, s.atividade, s.dt_saida, s.turno
           FROM saidas s
           JOIN veiculos v   ON v.id = s.veiculo_id
           JOIN motoristas m ON m.id = s.motorista_id
          WHERE s.dt_chegada IS NULL AND s.excluida_em IS NULL
          ORDER BY s.dt_saida",
    )?;

    let linhas = consulta.query_map([], |l| {
        Ok((
            l.get::<_, i64>(0)?,
            l.get::<_, String>(1)?,
            l.get::<_, String>(2)?,
            l.get::<_, Option<String>>(3)?,
            l.get::<_, String>(4)?,
            l.get::<_, String>(5)?,
            l.get::<_, String>(6)?,
        ))
    })?;

    let agora = chrono::Local::now().naive_local();
    let mut abertas = Vec::new();
    for linha in linhas {
        let (id, frota, motorista, destino, atividade, dt_saida, turno) = linha?;
        // Data ilegível não some do Painel: o carro está na rua de qualquer
        // jeito, e esconder a linha é pior que mostrar sem o tempo decorrido.
        let horas = datahora::analisar(&dt_saida)
            .map(|m| duracao::arredondar(duracao::horas_entre(m, agora)))
            .unwrap_or(0.0);

        abertas.push(SaidaAberta {
            id,
            frota,
            motorista,
            destino,
            atividade,
            dt_saida,
            turno,
            horas_decorridas: horas,
            alerta: horas > duracao::LIMIAR_ALERTA_ABERTA_HORAS,
        });
    }

    Ok(abertas)
}

pub fn listar_com(conexao: &Connection, filtro: &FiltroSaidas) -> Result<Vec<Saida>, ErroApp> {
    let (onde, parametros) = clausulas_do_filtro(filtro)?;
    let sql = format!(
        "SELECT {} {} {} ORDER BY s.dt_saida DESC",
        Saida::COLUNAS,
        Saida::JUNCOES,
        onde
    );

    let mut consulta = conexao.prepare(&sql)?;
    let referencias: Vec<&dyn rusqlite::ToSql> = parametros.iter().map(|p| p.as_ref()).collect();
    let linhas = consulta.query_map(referencias.as_slice(), Saida::da_linha)?;
    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

pub fn kpis_com(conexao: &Connection, filtro: &FiltroSaidas) -> Result<Kpis, ErroApp> {
    let (onde, parametros) = clausulas_do_filtro(filtro)?;
    let sql = format!(
        "SELECT COUNT(*),
                SUM(CASE WHEN s.dt_chegada IS NULL THEN 1 ELSE 0 END),
                COUNT(DISTINCT s.motorista_id),
                COUNT(DISTINCT s.veiculo_id),
                AVG(CASE WHEN s.dt_chegada IS NOT NULL
                         THEN (julianday(s.dt_chegada) - julianday(s.dt_saida)) * 24
                    END),
                SUM(CASE WHEN s.turno = 'A' THEN 1 ELSE 0 END),
                SUM(CASE WHEN s.turno = 'B' THEN 1 ELSE 0 END),
                SUM(CASE WHEN s.turno = 'C' THEN 1 ELSE 0 END)
           FROM saidas s {}",
        onde
    );

    let mut consulta = conexao.prepare(&sql)?;
    let referencias: Vec<&dyn rusqlite::ToSql> = parametros.iter().map(|p| p.as_ref()).collect();

    let kpis = consulta.query_row(referencias.as_slice(), |l| {
        Ok(Kpis {
            viagens: l.get::<_, i64>(0)? as u32,
            abertas: l.get::<_, Option<i64>>(1)?.unwrap_or(0) as u32,
            condutores_distintos: l.get::<_, i64>(2)? as u32,
            frotas_distintas: l.get::<_, i64>(3)? as u32,
            // `None` de propósito: média de zero viagens fechadas é ausência de
            // dado. Zero afirmaria que os carros rodaram sem gastar tempo.
            duracao_media_horas: l.get::<_, Option<f64>>(4)?.map(duracao::arredondar),
            por_turno: [
                l.get::<_, Option<i64>>(5)?.unwrap_or(0) as u32,
                l.get::<_, Option<i64>>(6)?.unwrap_or(0) as u32,
                l.get::<_, Option<i64>>(7)?.unwrap_or(0) as u32,
            ],
        })
    })?;

    Ok(kpis)
}

fn limpar(valor: Option<String>) -> Option<String> {
    valor
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

// --------------------------------------------------------------- comandos ---

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn abrir_saida(
    estado: State<'_, Estado>,
    veiculo_id: i64,
    motorista_id: i64,
    turno: String,
    destino: Option<String>,
    atividade: String,
    dt_saida: Option<String>,
    hodometro_saida: Option<i64>,
    observacao: Option<String>,
    confirmado_avisos: bool,
) -> Result<Saida, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    abrir_com(
        &mut conexao,
        DadosAbertura {
            veiculo_id,
            motorista_id,
            turno,
            destino,
            atividade,
            dt_saida,
            hodometro_saida,
            observacao,
        },
        confirmado_avisos,
    )
}

#[tauri::command]
pub fn encerrar_saida(
    estado: State<'_, Estado>,
    id: i64,
    dt_chegada: Option<String>,
    hodometro_chegada: Option<i64>,
    confirmado_avisos: bool,
) -> Result<Saida, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    encerrar_com(
        &mut conexao,
        id,
        dt_chegada,
        hodometro_chegada,
        confirmado_avisos,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn editar_saida(
    estado: State<'_, Estado>,
    id: i64,
    turno: String,
    destino: Option<String>,
    atividade: String,
    dt_saida: String,
    dt_chegada: Option<String>,
    hodometro_saida: Option<i64>,
    hodometro_chegada: Option<i64>,
    observacao: Option<String>,
    confirmado_avisos: bool,
) -> Result<Saida, ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;

    editar_com(
        &mut conexao,
        DadosEdicao {
            id,
            turno,
            destino,
            atividade,
            dt_saida,
            dt_chegada,
            hodometro_saida,
            hodometro_chegada,
            observacao,
        },
        confirmado_avisos,
    )
}

#[tauri::command]
pub fn excluir_saida(estado: State<'_, Estado>, id: i64, motivo: String) -> Result<(), ErroApp> {
    let sessao = estado.sessao()?;
    sessao.exigir_conexao_normal()?;
    let mut conexao = sessao.travar_conexao()?;
    excluir_com(&mut conexao, id, &motivo)
}

#[tauri::command]
pub fn listar_saidas_abertas(estado: State<'_, Estado>) -> Result<Vec<SaidaAberta>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    abertas_com(&conexao)
}

#[tauri::command]
pub fn listar_saidas(
    estado: State<'_, Estado>,
    filtro: FiltroSaidas,
) -> Result<Vec<Saida>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    listar_com(&conexao, &filtro)
}

#[tauri::command]
pub fn kpis(estado: State<'_, Estado>, filtro: FiltroSaidas) -> Result<Kpis, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    kpis_com(&conexao, &filtro)
}

/// Sugestões para o campo Destino, tiradas do que já foi digitado.
#[tauri::command]
pub fn destinos_recentes(estado: State<'_, Estado>) -> Result<Vec<String>, ErroApp> {
    let sessao = estado.sessao()?;
    let conexao = sessao.travar_conexao()?;
    let mut consulta = conexao.prepare(
        "SELECT destino FROM saidas
          WHERE destino IS NOT NULL AND TRIM(destino) <> '' AND excluida_em IS NULL
          GROUP BY destino
          ORDER BY COUNT(*) DESC, MAX(dt_saida) DESC
          LIMIT 30",
    )?;
    let linhas = consulta.query_map([], |l| l.get::<_, String>(0))?;
    Ok(linhas.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::db::{conexao, migracoes};
    use crate::dominio::nome::{frota_norm, nome_norm};

    /// Ontem e anteontem, calculados a partir de hoje.
    ///
    /// Data fixa aqui seria um teste que passa hoje e falha daqui a uma semana,
    /// quando a saída cruzar os 7 dias que pedem confirmação de atraso.
    fn dia(dias_atras: i64) -> String {
        datahora::formatar_data(datahora::hoje() - chrono::Duration::days(dias_atras))
    }

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
        for nome in ["Angelo Junior", "Jose Carlos", "Maria Lima"] {
            c.execute(
                "INSERT INTO motoristas (nome, nome_norm) VALUES (?1, ?2)",
                rusqlite::params![nome, nome_norm(nome)],
            )
            .expect("cria condutor");
        }
        c
    }

    fn abertura(veiculo_id: i64, motorista_id: i64) -> DadosAbertura {
        DadosAbertura {
            veiculo_id,
            motorista_id,
            turno: "A".to_string(),
            destino: Some("Campo 7".to_string()),
            atividade: "Transporte de equipe".to_string(),
            dt_saida: Some(format!("{} 08:00", dia(1))),
            hodometro_saida: None,
            observacao: None,
        }
    }

    #[test]
    fn segunda_abertura_do_mesmo_veiculo_devolve_veiculo_em_uso_e_nao_banco() {
        let mut c = base();
        abrir_com(&mut c, abertura(1, 1), false).expect("primeira abre");

        let erro = abrir_com(&mut c, abertura(1, 2), false).expect_err("segunda deve recusar");

        assert_eq!(erro.codigo, "VEICULO_EM_USO");
        assert!(
            erro.detalhe.unwrap_or_default().contains("Angelo Junior"),
            "a recusa precisa dizer quem está com o carro"
        );
    }

    #[test]
    fn segunda_abertura_do_mesmo_motorista_devolve_motorista_em_uso() {
        let mut c = base();
        abrir_com(&mut c, abertura(1, 1), false).expect("primeira abre");

        let erro = abrir_com(&mut c, abertura(2, 1), false).expect_err("segunda deve recusar");
        assert_eq!(erro.codigo, "MOTORISTA_EM_USO");
    }

    /// O `SELECT` prévio é conveniência. Quem garante a regra é o índice único
    /// parcial, e este teste fura o `SELECT` para provar que ele segura sozinho.
    #[test]
    fn o_indice_unico_barra_mesmo_sem_o_select_previo() {
        let c = base();
        c.execute(
            "INSERT INTO saidas (veiculo_id, motorista_id, turno, atividade, dt_saida)
             VALUES (1, 1, 'A', 'Teste', ?1)",
            [format!("{} 08:00", dia(1))],
        )
        .expect("primeira entra");

        let erro = c
            .execute(
                "INSERT INTO saidas (veiculo_id, motorista_id, turno, atividade, dt_saida)
                 VALUES (1, 2, 'A', 'Teste', ?1)",
                [format!("{} 09:00", dia(1))],
            )
            .expect_err("o índice deve barrar");

        assert_eq!(ErroApp::from(erro).codigo, "VEICULO_EM_USO");
    }

    #[test]
    fn excluir_viagem_aberta_libera_o_indice_para_o_mesmo_veiculo() {
        let mut c = base();
        let primeira = abrir_com(&mut c, abertura(1, 1), false).expect("abre");

        excluir_com(&mut c, primeira.id, "lançamento em duplicidade").expect("exclui");

        let segunda = abrir_com(&mut c, abertura(1, 2), false)
            .expect("com a primeira excluída, o veículo está livre");
        assert!(segunda.dt_chegada.is_none());

        let visiveis = listar_com(&c, &FiltroSaidas::default()).expect("lista");
        assert_eq!(visiveis.len(), 1, "a excluída não aparece por padrão");

        let com_excluidas = listar_com(
            &c,
            &FiltroSaidas {
                incluir_excluidas: true,
                ..Default::default()
            },
        )
        .expect("lista");
        assert_eq!(com_excluidas.len(), 2);
        assert!(com_excluidas
            .iter()
            .any(|s| s.excluida_motivo.as_deref() == Some("lançamento em duplicidade")));
    }

    #[test]
    fn exclusao_sem_motivo_e_recusada() {
        let mut c = base();
        let saida = abrir_com(&mut c, abertura(1, 1), false).expect("abre");

        let erro = excluir_com(&mut c, saida.id, "  ").expect_err("deve recusar");
        assert_eq!(erro.codigo, "VALIDACAO");

        let ainda_aberta = por_id(&c, saida.id).expect("lê");
        assert!(ainda_aberta.excluida_em.is_none());
    }

    #[test]
    fn encerrar_libera_o_veiculo_para_a_proxima_viagem() {
        let mut c = base();
        let primeira = abrir_com(&mut c, abertura(1, 1), false).expect("abre");

        encerrar_com(
            &mut c,
            primeira.id,
            Some(format!("{} 12:00", dia(1))),
            None,
            false,
        )
        .expect("encerra");

        abrir_com(&mut c, abertura(1, 2), false).expect("veículo livre de novo");
    }

    #[test]
    fn encerramento_longo_exige_confirmacao_e_passa_quando_confirmado() {
        let mut c = base();
        let saida = abrir_com(&mut c, abertura(1, 1), false).expect("abre");

        let erro = encerrar_com(
            &mut c,
            saida.id,
            Some(format!("{} 07:00", dia(0))),
            None,
            false,
        )
        .expect_err("23 h precisa de confirmação");

        assert_eq!(erro.codigo, "VALIDACAO");
        assert_eq!(erro.detalhe.as_deref(), Some("DURACAO_LONGA:23h00"));

        let encerrada = encerrar_com(
            &mut c,
            saida.id,
            Some(format!("{} 07:00", dia(0))),
            None,
            true,
        )
        .expect("confirmado, passa");
        assert_eq!(encerrada.dt_chegada, Some(format!("{} 07:00", dia(0))));
        assert!(encerrada.chegada_manual, "horário informado à mão");
    }

    #[test]
    fn virada_de_meia_noite_encerra_sem_confirmacao() {
        let mut c = base();
        let mut d = abertura(1, 1);
        d.dt_saida = Some(format!("{} 22:40", dia(1)));
        let saida = abrir_com(&mut c, d, false).expect("abre");

        encerrar_com(
            &mut c,
            saida.id,
            Some(format!("{} 05:30", dia(0))),
            None,
            false,
        )
        .expect("6h50 não pede confirmação");
    }

    #[test]
    fn hodometro_de_chegada_menor_que_o_de_saida_e_recusado() {
        let mut c = base();
        let mut d = abertura(1, 1);
        d.hodometro_saida = Some(45_200);
        let saida = abrir_com(&mut c, d, false).expect("abre");

        let erro = encerrar_com(
            &mut c,
            saida.id,
            Some(format!("{} 12:00", dia(1))),
            Some(44_900),
            true,
        )
        .expect_err("deve recusar");
        assert_eq!(erro.codigo, "VALIDACAO");
    }

    #[test]
    fn encerrar_avanca_o_hodometro_do_veiculo_mas_nunca_para_tras() {
        let mut c = base();
        let mut d = abertura(1, 1);
        d.hodometro_saida = Some(45_000);
        let saida = abrir_com(&mut c, d, false).expect("abre");

        encerrar_com(
            &mut c,
            saida.id,
            Some(format!("{} 12:00", dia(1))),
            Some(45_300),
            false,
        )
        .expect("encerra");

        let atual: Option<i64> = c
            .query_row(
                "SELECT hodometro_atual FROM veiculos WHERE id = 1",
                [],
                |l| l.get(0),
            )
            .expect("lê");
        assert_eq!(atual, Some(45_300));

        let mut d2 = abertura(1, 2);
        d2.dt_saida = Some(format!("{} 13:00", dia(1)));
        d2.hodometro_saida = Some(45_300);
        let segunda = abrir_com(&mut c, d2, false).expect("abre");
        encerrar_com(
            &mut c,
            segunda.id,
            Some(format!("{} 14:00", dia(1))),
            Some(45_100),
            true,
        )
        .expect_err("chegada menor que a saída é recusada");

        let depois: Option<i64> = c
            .query_row(
                "SELECT hodometro_atual FROM veiculos WHERE id = 1",
                [],
                |l| l.get(0),
            )
            .expect("lê");
        assert_eq!(depois, Some(45_300), "o hodômetro não pode retroceder");
    }

    #[test]
    fn viagem_aberta_conta_como_viagem_mas_nao_soma_horas() {
        let mut c = base();
        let fechada = abrir_com(&mut c, abertura(1, 1), false).expect("abre");
        encerrar_com(
            &mut c,
            fechada.id,
            Some(format!("{} 12:00", dia(1))),
            None,
            false,
        )
        .expect("encerra");
        abrir_com(&mut c, abertura(2, 2), false).expect("abre a segunda");

        let k = kpis_com(&c, &FiltroSaidas::default()).expect("kpis");
        assert_eq!(k.viagens, 2);
        assert_eq!(k.abertas, 1);
        assert_eq!(
            k.duracao_media_horas,
            Some(4.0),
            "a média usa só a viagem fechada"
        );
    }

    #[test]
    fn media_de_zero_viagens_fechadas_e_ausencia_de_dado_e_nao_zero() {
        let mut c = base();
        abrir_com(&mut c, abertura(1, 1), false).expect("abre");

        let k = kpis_com(&c, &FiltroSaidas::default()).expect("kpis");
        assert_eq!(k.viagens, 1);
        assert_eq!(k.duracao_media_horas, None);
    }

    #[test]
    fn auditoria_registra_abertura_encerramento_e_exclusao() {
        let mut c = base();
        let saida = abrir_com(&mut c, abertura(1, 1), false).expect("abre");
        encerrar_com(
            &mut c,
            saida.id,
            Some(format!("{} 12:00", dia(1))),
            None,
            false,
        )
        .expect("encerra");
        excluir_com(&mut c, saida.id, "teste de auditoria").expect("exclui");

        let mut consulta = c
            .prepare("SELECT acao FROM auditoria ORDER BY id")
            .expect("consulta");
        let acoes: Vec<String> = consulta
            .query_map([], |l| l.get(0))
            .expect("lê")
            .collect::<Result<_, _>>()
            .expect("coleta");

        assert_eq!(
            acoes,
            vec![
                auditoria::ABRIR_SAIDA,
                auditoria::ENCERRAR_SAIDA,
                auditoria::EXCLUIR_SAIDA
            ]
        );
    }

    #[test]
    fn saida_no_futuro_alem_da_folga_e_recusada() {
        let mut c = base();
        let mut d = abertura(1, 1);
        let amanha = chrono::Local::now().naive_local() + chrono::Duration::days(1);
        d.dt_saida = Some(datahora::formatar(amanha));

        let erro = abrir_com(&mut c, d, true).expect_err("futuro é recusado");
        assert_eq!(erro.codigo, "VALIDACAO");
    }

    #[test]
    fn abrir_para_veiculo_inexistente_devolve_nao_encontrado() {
        let mut c = base();
        let erro = abrir_com(&mut c, abertura(99, 1), false).expect_err("não existe");
        assert_eq!(erro.codigo, "NAO_ENCONTRADO");
    }
}
