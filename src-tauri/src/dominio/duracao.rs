//! Duração de viagem.
//!
//! As datas são ISO completas, com dia. Não existe adivinhação de virada de
//! meia-noite: `22:40 → 05:30` do dia seguinte são 6,83 h por subtração direta,
//! e não 16 h nem um alerta espúrio.

use chrono::NaiveDateTime;

/// Existem turnos legítimos de monitor com 10 a 12 h. O alerta não pode
/// disparar antes disso, ou vira ruído que o operador aprende a ignorar.
pub const LIMIAR_DURACAO_LONGA_HORAS: f64 = 14.0;

/// Acima disso, a linha da viagem aberta fica vermelha no Painel.
pub const LIMIAR_ALERTA_ABERTA_HORAS: f64 = 24.0;

pub fn horas_entre(saida: NaiveDateTime, chegada: NaiveDateTime) -> f64 {
    let segundos = (chegada - saida).num_seconds() as f64;
    segundos / 3600.0
}

/// Duas casas, como os relatórios arredondam. Comparar o valor cru com o
/// exibido é o que gera "o relatório diz 6,83 e a tela diz 6,8333".
pub fn arredondar(horas: f64) -> f64 {
    (horas * 100.0).round() / 100.0
}

pub fn longa(horas: f64) -> bool {
    horas > LIMIAR_DURACAO_LONGA_HORAS
}

/// `16h20`, que é como a mensagem de confirmação precisa mostrar.
pub fn formatar(horas: f64) -> String {
    let total_minutos = (horas * 60.0).round() as i64;
    let h = total_minutos / 60;
    let m = (total_minutos % 60).abs();
    format!("{h}h{m:02}")
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::dominio::datahora::analisar;

    fn horas(saida: &str, chegada: &str) -> f64 {
        let s = analisar(saida).expect("data válida");
        let c = analisar(chegada).expect("data válida");
        arredondar(horas_entre(s, c))
    }

    #[test]
    fn virada_de_meia_noite_da_seis_horas_e_cinquenta_minutos() {
        let h = horas("2026-09-19 22:40", "2026-09-20 05:30");
        assert_eq!(h, 6.83);
        assert!(!longa(h));
        assert_eq!(formatar(h), "6h50");
    }

    #[test]
    fn oito_da_manha_ate_sete_do_dia_seguinte_e_longa() {
        let h = horas("2026-09-19 08:00", "2026-09-20 07:00");
        assert_eq!(h, 23.0);
        assert!(longa(h));
        assert_eq!(formatar(h), "23h00");
    }

    #[test]
    fn turno_de_monitor_de_doze_horas_nao_alerta() {
        let h = horas("2026-09-19 06:00", "2026-09-19 18:00");
        assert_eq!(h, 12.0);
        assert!(!longa(h));
    }

    #[test]
    fn catorze_horas_cravadas_nao_alertam_e_quatorze_e_um_minuto_alerta() {
        assert!(!longa(horas("2026-09-19 06:00", "2026-09-19 20:00")));
        assert!(longa(horas("2026-09-19 06:00", "2026-09-19 20:01")));
    }

    #[test]
    fn formata_dezesseis_horas_e_vinte() {
        assert_eq!(formatar(16.3333), "16h20");
    }
}
