//! Base fictícia da apresentação.
//!
//! É o entregável de verdade do modo demonstração: tela vazia não apresenta
//! nada, e dado fictício mal feito apresenta pior ainda. Cada coisa aqui existe
//! para que uma parte do roteiro tenha o que mostrar.

use chrono::{Duration, NaiveDateTime, Timelike};
use rusqlite::{params, Connection};

use crate::dominio::datahora;
use crate::dominio::nome::{frota_norm, nome_norm};
use crate::erro::ErroApp;
use crate::registro;

const VEICULOS: &[(&str, &str, &str, i64, &str)] = &[
    ("907015", "Saveiro", "Volkswagen", 2021, "Utilitário"),
    ("907018", "Strada", "Fiat", 2022, "Utilitário"),
    ("907022", "Hilux", "Toyota", 2020, "Caminhonete"),
    ("907031", "Ranger", "Ford", 2019, "Caminhonete"),
    ("907044", "Kombi", "Volkswagen", 2015, "Van"),
    ("907050", "Ducato", "Fiat", 2023, "Van"),
    ("907061", "Onix", "Chevrolet", 2022, "Passeio"),
    ("907073", "Gol", "Volkswagen", 2018, "Passeio"),
];

/// `ANGELO JUNIOR` e `ANJELO JUNIOR` são propositais: é com eles que o alerta de
/// nome parecido tem o que alertar na apresentação.
const MOTORISTAS: &[(&str, &str, &str)] = &[
    ("Angelo Junior", "40122", "Logística"),
    ("Anjelo Junior", "40871", "Logística"),
    ("José Carlos da Silva", "40233", "Logística"),
    ("Maria das Graças Lima", "40344", "Logística"),
    ("Antônio Ferreira", "40455", "Manutenção"),
    ("Cícero Nascimento", "40566", "Logística"),
    ("Rosângela Alves", "40677", "Administrativo"),
    ("Paulo Sérgio Rocha", "40788", "Logística"),
    ("Francisco de Assis", "40899", "Manutenção"),
    ("Luciana Cavalcanti", "40911", "Logística"),
];

const DESTINOS: &[&str] = &[
    "Portaria principal",
    "Campo 7",
    "Oficina central",
    "Almoxarifado",
    "Balança",
    "Escritório da usina",
    "Recife — matriz",
    "Destilaria",
];

const ATIVIDADES: &[&str] = &[
    "Transporte de equipe",
    "Entrega de material",
    "Vistoria de campo",
    "Apoio à manutenção",
    "Coleta de amostras",
    "Transporte administrativo",
];

/// Gerador determinístico: a mesma apresentação duas vezes mostra os mesmos
/// números. Um `rand` traria dependência nova só para a demo, e sorteio
/// diferente a cada abertura atrapalha quem está ensaiando a fala.
struct Sorteio(u64);

impl Sorteio {
    fn proximo(&mut self, limite: u64) -> u64 {
        // Xorshift: previsível, suficiente para espalhar dado fictício.
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % limite.max(1)
    }
}

pub fn popular(conexao: &Connection) -> Result<(), ErroApp> {
    registro::info("gerando base fictícia da demonstração");

    for (frota, modelo, marca, ano, tipo) in VEICULOS {
        conexao.execute(
            "INSERT INTO veiculos (frota, modelo, marca, ano, tipo, hodometro_atual)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                frota_norm(frota),
                modelo,
                marca,
                ano,
                tipo,
                40_000 + (ano - 2015) * 12_000
            ],
        )?;
    }

    for (nome, matricula, setor) in MOTORISTAS {
        conexao.execute(
            "INSERT INTO motoristas (nome, nome_norm, matricula, setor) VALUES (?1, ?2, ?3, ?4)",
            params![nome, nome_norm(nome), matricula, setor],
        )?;
    }

    let agora = chrono::Local::now().naive_local();
    let mut sorteio = Sorteio(0x5EED_1234_ABCD_0001);

    // Dois meses de histórico fechado. Sem isso o relatório do mês sai vazio, e
    // relatório é o que convence quem assina a liberação.
    let mut fechadas = 0;
    for dias_atras in (2..=60).rev() {
        let quantas = 1 + sorteio.proximo(3);
        for _ in 0..quantas {
            let veiculo = 1 + sorteio.proximo(VEICULOS.len() as u64) as i64;
            let motorista = 1 + sorteio.proximo(MOTORISTAS.len() as u64) as i64;
            let hora = 6 + sorteio.proximo(12) as u32;
            let minuto = (sorteio.proximo(4) * 15) as u32;

            let Some(saida) = (agora - Duration::days(dias_atras))
                .with_hour(hora)
                .and_then(|d| d.with_minute(minuto))
                .and_then(|d| d.with_second(0))
            else {
                continue;
            };

            let duracao_minutos = 40 + sorteio.proximo(420) as i64;
            let chegada = saida + Duration::minutes(duracao_minutos);
            let manual = sorteio.proximo(4) == 0;
            let com_hodometro = sorteio.proximo(3) != 0;

            let hodometro_saida = com_hodometro.then(|| 40_000 + sorteio.proximo(30_000) as i64);
            let hodometro_chegada = hodometro_saida.map(|h| h + 8 + sorteio.proximo(180) as i64);

            inserir(
                conexao,
                veiculo,
                motorista,
                &saida,
                Some(&chegada),
                manual,
                hodometro_saida,
                hodometro_chegada,
                &mut sorteio,
            )?;
            fechadas += 1;
        }
    }

    // Viagens abertas, uma por veículo distinto: são as linhas do Painel, e o
    // índice único não deixa repetir veículo nem condutor.
    let abertas: [(i64, i64, i64); 4] = [
        (1, 1, 3),  // 3 h na rua
        (2, 3, 9),  // 9 h
        (3, 4, 26), // > 24 h: é esta que pinta a linha de vermelho
        (5, 6, 1),  // 1 h
    ];

    for (veiculo, motorista, horas) in abertas {
        let saida = agora - Duration::hours(horas);
        let hodometro = 40_000 + sorteio.proximo(30_000) as i64;
        inserir(
            conexao,
            veiculo,
            motorista,
            &saida,
            None,
            false,
            Some(hodometro),
            None,
            &mut sorteio,
        )?;
    }

    registro::info(&format!(
        "base fictícia pronta: {fechadas} viagens fechadas e {} abertas",
        abertas.len()
    ));

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn inserir(
    conexao: &Connection,
    veiculo_id: i64,
    motorista_id: i64,
    saida: &NaiveDateTime,
    chegada: Option<&NaiveDateTime>,
    chegada_manual: bool,
    hodometro_saida: Option<i64>,
    hodometro_chegada: Option<i64>,
    sorteio: &mut Sorteio,
) -> Result<(), ErroApp> {
    let turno = match saida.hour() {
        0..=6 => "C",
        7..=14 => "A",
        _ => "B",
    };
    let destino = DESTINOS[sorteio.proximo(DESTINOS.len() as u64) as usize];
    let atividade = ATIVIDADES[sorteio.proximo(ATIVIDADES.len() as u64) as usize];

    conexao.execute(
        "INSERT INTO saidas
             (veiculo_id, motorista_id, turno, destino, atividade, dt_saida, dt_chegada,
              hodometro_saida, hodometro_chegada, chegada_manual)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            veiculo_id,
            motorista_id,
            turno,
            destino,
            atividade,
            datahora::formatar(*saida),
            chegada.map(|c| datahora::formatar(*c)),
            hodometro_saida,
            hodometro_chegada,
            i64::from(chegada_manual)
        ],
    )?;

    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::db::{conexao, migracoes};
    use crate::modelo::FiltroSaidas;

    fn base_demo() -> Connection {
        let c = conexao::abrir_em_memoria().expect("abre");
        migracoes::aplicar(&c, |_| Ok(())).expect("migra");
        popular(&c).expect("popula");
        c
    }

    #[test]
    fn a_base_ficticia_tem_viagens_abertas_e_fechadas() {
        let c = base_demo();

        let abertas: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM saidas WHERE dt_chegada IS NULL AND excluida_em IS NULL",
                [],
                |l| l.get(0),
            )
            .expect("conta");
        let fechadas: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM saidas WHERE dt_chegada IS NOT NULL",
                [],
                |l| l.get(0),
            )
            .expect("conta");

        assert_eq!(abertas, 4);
        assert!(fechadas > 50, "dois meses precisam ter volume: {fechadas}");
    }

    #[test]
    fn existe_uma_viagem_passando_de_vinte_e_quatro_horas() {
        let c = base_demo();
        let abertas = crate::comandos::saidas::abertas_com(&c).expect("lista");
        assert!(
            abertas.iter().any(|a| a.alerta),
            "a demo precisa de uma linha vermelha para mostrar o alerta"
        );
    }

    #[test]
    fn existem_dois_nomes_parecidos_para_o_alerta_de_duplicata() {
        let c = base_demo();
        let a = nome_norm("Angelo Junior");
        let b = nome_norm("Anjelo Junior");
        assert!(crate::dominio::semelhanca::nomes_semelhantes(&a, &b));

        let existem: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM motoristas WHERE nome_norm IN (?1, ?2)",
                [&a, &b],
                |l| l.get(0),
            )
            .expect("conta");
        assert_eq!(existem, 2);
    }

    #[test]
    fn os_kpis_da_demo_nao_ficam_zerados() {
        let c = base_demo();
        let kpis = crate::comandos::saidas::kpis_com(&c, &FiltroSaidas::default()).expect("kpis");

        assert!(kpis.viagens > 50);
        assert_eq!(kpis.abertas, 4);
        assert!(kpis.duracao_media_horas.is_some(), "há viagens fechadas");
        assert!(kpis.por_turno.iter().sum::<u32>() == kpis.viagens);
    }

    #[test]
    fn o_gerador_e_deterministico() {
        let primeira = base_demo();
        let segunda = base_demo();

        let conta = |c: &Connection| -> i64 {
            c.query_row("SELECT COUNT(*) FROM saidas", [], |l| l.get(0))
                .expect("conta")
        };
        assert_eq!(conta(&primeira), conta(&segunda));
    }
}
