//! As tabelas da seção 6 do SPEC, em funções puras.
//!
//! Duas categorias, e a diferença importa: **recusa** é erro, o registro não
//! entra; **aviso** é confirmação, o registro entra depois que o operador diz
//! que é isso mesmo. Transformar aviso em recusa trava o turno legítimo de
//! monitor; transformar recusa em aviso deixa o `08:00 → 07:00` digitado errado
//! passar.

use chrono::NaiveDateTime;

use crate::dominio::datahora;
use crate::dominio::duracao;
use crate::erro::ErroApp;

pub const FOLGA_FUTURO_MINUTOS: i64 = 5;
pub const DIAS_SAIDA_ANTIGA: i64 = 7;
pub const DIFERENCA_HODOMETRO_CONFIRMA_KM: i64 = 1_000;
pub const NOME_MINIMO: usize = 3;
pub const FROTA_MINIMO: usize = 4;
pub const FROTA_MAXIMO: usize = 8;
pub const MOTIVO_MINIMO: usize = 3;

pub const TURNOS: [&str; 3] = ["A", "B", "C"];

/// O que exige um "é isso mesmo?" antes de gravar.
#[derive(Debug, Clone, PartialEq)]
pub enum Aviso {
    SaidaAntiga { dias: i64 },
    HodometroMenorQueAtual { registrado: i64, informado: i64 },
    DuracaoLonga { horas: f64 },
    DiferencaHodometroGrande { km: i64 },
}

impl Aviso {
    /// Vai em `ErroApp::detalhe`. O React quebra no `:` e decide o texto do
    /// diálogo pelo prefixo, que é estável.
    pub fn detalhe(&self) -> String {
        match self {
            Aviso::SaidaAntiga { dias } => format!("SAIDA_ANTIGA:{dias}"),
            Aviso::HodometroMenorQueAtual {
                registrado,
                informado,
            } => format!("HODOMETRO_MENOR:{registrado}:{informado}"),
            Aviso::DuracaoLonga { horas } => {
                format!("DURACAO_LONGA:{}", duracao::formatar(*horas))
            }
            Aviso::DiferencaHodometroGrande { km } => format!("HODOMETRO_DIFERENCA:{km}"),
        }
    }

    pub fn mensagem(&self) -> String {
        match self {
            Aviso::SaidaAntiga { dias } => {
                format!("A saída está sendo registrada com {dias} dias de atraso.")
            }
            Aviso::HodometroMenorQueAtual {
                registrado,
                informado,
            } => format!("Hodômetro registrado {registrado}, informado {informado}."),
            Aviso::DuracaoLonga { horas } => format!(
                "A viagem durou {}. Confirma esse horário de chegada?",
                duracao::formatar(*horas)
            ),
            Aviso::DiferencaHodometroGrande { km } => {
                format!("A viagem teria {km} km. Confirma o hodômetro?")
            }
        }
    }
}

/// Junta os avisos num único `VALIDACAO` para a tela perguntar de uma vez.
pub fn erro_de_confirmacao(avisos: &[Aviso]) -> ErroApp {
    let mensagem = avisos
        .iter()
        .map(Aviso::mensagem)
        .collect::<Vec<_>>()
        .join(" ");
    let detalhe = avisos
        .iter()
        .map(Aviso::detalhe)
        .collect::<Vec<_>>()
        .join(";");
    ErroApp::validacao(mensagem).com_detalhe(detalhe)
}

pub fn validar_turno(turno: &str) -> Result<(), ErroApp> {
    if TURNOS.contains(&turno) {
        return Ok(());
    }
    Err(ErroApp::validacao("Turno deve ser A, B ou C.").com_detalhe("CAMPO:turno"))
}

pub fn validar_atividade(atividade: &str) -> Result<(), ErroApp> {
    if atividade.trim().is_empty() {
        return Err(ErroApp::validacao("Informe a atividade.").com_detalhe("CAMPO:atividade"));
    }
    Ok(())
}

pub fn validar_nome(nome: &str) -> Result<(), ErroApp> {
    if nome.trim().chars().count() < NOME_MINIMO {
        return Err(ErroApp::validacao(format!(
            "O nome precisa ter pelo menos {NOME_MINIMO} caracteres."
        ))
        .com_detalhe("CAMPO:nome"));
    }
    Ok(())
}

/// Recebe a frota **já normalizada** por [`super::nome::frota_norm`].
pub fn validar_frota(frota: &str) -> Result<(), ErroApp> {
    let total = frota.chars().count();
    if !(FROTA_MINIMO..=FROTA_MAXIMO).contains(&total)
        || !frota.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Err(ErroApp::validacao(format!(
            "A frota precisa ter de {FROTA_MINIMO} a {FROTA_MAXIMO} caracteres, \
             apenas letras e números."
        ))
        .com_detalhe("CAMPO:frota"));
    }
    Ok(())
}

pub struct DadosAbertura {
    pub turno: String,
    pub atividade: String,
    pub dt_saida: NaiveDateTime,
    pub hodometro_saida: Option<i64>,
    pub veiculo_ativo: bool,
    pub motorista_ativo: bool,
    pub hodometro_atual: Option<i64>,
}

pub fn validar_abertura(d: &DadosAbertura) -> Result<Vec<Aviso>, ErroApp> {
    if !d.veiculo_ativo {
        return Err(ErroApp::validacao("Este veículo está inativo.").com_detalhe("CAMPO:veiculo"));
    }
    if !d.motorista_ativo {
        return Err(
            ErroApp::validacao("Este condutor está inativo.").com_detalhe("CAMPO:motorista")
        );
    }
    validar_turno(&d.turno)?;
    validar_atividade(&d.atividade)?;

    if datahora::no_futuro_alem_de(d.dt_saida, FOLGA_FUTURO_MINUTOS) {
        return Err(
            ErroApp::validacao("A saída não pode ser registrada no futuro.")
                .com_detalhe("CAMPO:dt_saida"),
        );
    }

    let mut avisos = Vec::new();

    let dias = datahora::dias_atras(d.dt_saida);
    if dias > DIAS_SAIDA_ANTIGA {
        avisos.push(Aviso::SaidaAntiga { dias });
    }

    if let (Some(informado), Some(registrado)) = (d.hodometro_saida, d.hodometro_atual) {
        if informado < registrado {
            avisos.push(Aviso::HodometroMenorQueAtual {
                registrado,
                informado,
            });
        }
    }

    Ok(avisos)
}

pub struct DadosEncerramento {
    pub dt_saida: NaiveDateTime,
    pub dt_chegada: NaiveDateTime,
    pub ja_encerrada: bool,
    pub hodometro_saida: Option<i64>,
    pub hodometro_chegada: Option<i64>,
}

pub fn validar_encerramento(d: &DadosEncerramento) -> Result<Vec<Aviso>, ErroApp> {
    if d.ja_encerrada {
        return Err(ErroApp::validacao("Esta viagem já foi encerrada."));
    }
    if d.dt_chegada <= d.dt_saida {
        return Err(ErroApp::validacao("A chegada precisa ser depois da saída.")
            .com_detalhe("CAMPO:dt_chegada"));
    }
    if datahora::no_futuro_alem_de(d.dt_chegada, FOLGA_FUTURO_MINUTOS) {
        return Err(
            ErroApp::validacao("A chegada não pode ser registrada no futuro.")
                .com_detalhe("CAMPO:dt_chegada"),
        );
    }

    if let (Some(chegada), Some(saida)) = (d.hodometro_chegada, d.hodometro_saida) {
        if chegada < saida {
            return Err(ErroApp::validacao(
                "O hodômetro de chegada não pode ser menor que o de saída.",
            )
            .com_detalhe("CAMPO:hodometro_chegada"));
        }
    }

    let mut avisos = Vec::new();

    let horas = duracao::arredondar(duracao::horas_entre(d.dt_saida, d.dt_chegada));
    if duracao::longa(horas) {
        avisos.push(Aviso::DuracaoLonga { horas });
    }

    if let (Some(chegada), Some(saida)) = (d.hodometro_chegada, d.hodometro_saida) {
        let km = chegada - saida;
        if km > DIFERENCA_HODOMETRO_CONFIRMA_KM {
            avisos.push(Aviso::DiferencaHodometroGrande { km });
        }
    }

    Ok(avisos)
}

pub fn validar_fusao(
    manter_id: i64,
    remover_id: i64,
    manter_tem_aberta: bool,
    remover_tem_aberta: bool,
) -> Result<(), ErroApp> {
    if manter_id == remover_id {
        return Err(ErroApp::validacao(
            "Selecione dois cadastros diferentes para fundir.",
        ));
    }
    // Reapontar saídas com viagem aberta dos dois lados estoura o índice único
    // no meio da transação, e o erro que chega à tela não explica nada.
    if manter_tem_aberta || remover_tem_aberta {
        return Err(ErroApp::validacao(
            "Encerre as viagens abertas dos dois cadastros antes de fundir.",
        ));
    }
    Ok(())
}

pub fn validar_exclusao(ja_excluida: bool, motivo: &str) -> Result<(), ErroApp> {
    if ja_excluida {
        return Err(ErroApp::validacao("Esta viagem já foi excluída."));
    }
    if motivo.trim().chars().count() < MOTIVO_MINIMO {
        return Err(ErroApp::validacao(format!(
            "Informe o motivo da exclusão, com pelo menos {MOTIVO_MINIMO} caracteres."
        ))
        .com_detalhe("CAMPO:motivo"));
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::dominio::datahora::analisar;
    use crate::dominio::nome::frota_norm;

    fn encerramento(saida: &str, chegada: &str) -> DadosEncerramento {
        DadosEncerramento {
            dt_saida: analisar(saida).expect("data válida"),
            dt_chegada: analisar(chegada).expect("data válida"),
            ja_encerrada: false,
            hodometro_saida: None,
            hodometro_chegada: None,
        }
    }

    #[test]
    fn duracao_longa_devolve_o_aviso_formatado() {
        let avisos = validar_encerramento(&encerramento("2026-09-19 08:00", "2026-09-20 00:20"))
            .expect("ok");
        assert_eq!(avisos, vec![Aviso::DuracaoLonga { horas: 16.33 }]);
        assert_eq!(avisos[0].detalhe(), "DURACAO_LONGA:16h20");
    }

    #[test]
    fn virada_de_meia_noite_nao_gera_aviso() {
        let avisos = validar_encerramento(&encerramento("2026-09-19 22:40", "2026-09-20 05:30"))
            .expect("ok");
        assert!(avisos.is_empty());
    }

    #[test]
    fn chegada_antes_da_saida_e_recusa() {
        let erro = validar_encerramento(&encerramento("2026-09-19 08:00", "2026-09-19 07:00"))
            .expect_err("deve recusar");
        assert_eq!(erro.codigo, "VALIDACAO");
    }

    #[test]
    fn hodometro_de_chegada_menor_e_recusa() {
        let mut d = encerramento("2026-09-19 08:00", "2026-09-19 09:00");
        d.hodometro_saida = Some(45_200);
        d.hodometro_chegada = Some(44_900);
        assert!(validar_encerramento(&d).is_err());
    }

    #[test]
    fn diferenca_de_hodometro_grande_so_confirma() {
        let mut d = encerramento("2026-09-19 08:00", "2026-09-19 12:00");
        d.hodometro_saida = Some(45_200);
        d.hodometro_chegada = Some(46_500);
        let avisos = validar_encerramento(&d).expect("não deve recusar");
        assert_eq!(avisos, vec![Aviso::DiferencaHodometroGrande { km: 1_300 }]);
    }

    #[test]
    fn viagem_ja_encerrada_e_recusa() {
        let mut d = encerramento("2026-09-19 08:00", "2026-09-19 09:00");
        d.ja_encerrada = true;
        assert!(validar_encerramento(&d).is_err());
    }

    #[test]
    fn frota_curta_e_frota_com_simbolo_sao_recusadas() {
        assert!(validar_frota(&frota_norm("907")).is_err());
        assert!(validar_frota(&frota_norm("907015123")).is_err());
        assert!(validar_frota(&frota_norm("AB-1234")).is_err());
        assert!(validar_frota(&frota_norm("907015")).is_ok());
    }

    #[test]
    fn nome_com_duas_letras_e_recusado() {
        assert!(validar_nome("Jo").is_err());
        assert!(validar_nome("Ana").is_ok());
    }

    #[test]
    fn motivo_de_exclusao_vazio_ou_curto_e_recusado() {
        assert!(validar_exclusao(false, "").is_err());
        assert!(validar_exclusao(false, "  x ").is_err());
        assert!(validar_exclusao(false, "erro de digitação").is_ok());
    }

    #[test]
    fn fusao_do_mesmo_id_ou_com_viagem_aberta_e_recusada() {
        assert!(validar_fusao(1, 1, false, false).is_err());
        assert!(validar_fusao(1, 2, true, false).is_err());
        assert!(validar_fusao(1, 2, false, true).is_err());
        assert!(validar_fusao(1, 2, false, false).is_ok());
    }

    #[test]
    fn turno_fora_de_abc_e_recusado() {
        assert!(validar_turno("D").is_err());
        assert!(validar_turno("A").is_ok());
    }
}
