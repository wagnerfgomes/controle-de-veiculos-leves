//! Data-hora do sistema: `YYYY-MM-DD HH:MM`, sempre local.
//!
//! Nenhuma função aqui aceita UTC ou epoch. O setor inteiro opera num fuso só,
//! e converter só criaria chance de erro num campo que o operador digita à mão.

use chrono::{Duration, Local, NaiveDate, NaiveDateTime};

use crate::erro::ErroApp;

pub const FORMATO: &str = "%Y-%m-%d %H:%M";
const FORMATO_COM_SEGUNDOS: &str = "%Y-%m-%d %H:%M:%S";
pub const FORMATO_DATA: &str = "%Y-%m-%d";

pub fn agora() -> String {
    formatar(Local::now().naive_local())
}

pub fn agora_com_segundos() -> String {
    Local::now().format(FORMATO_COM_SEGUNDOS).to_string()
}

/// Usado no nome de arquivo de backup, onde `:` é inválido no Windows.
pub fn agora_para_arquivo() -> String {
    Local::now().format("%Y-%m-%d_%H%M").to_string()
}

pub fn hoje() -> NaiveDate {
    Local::now().date_naive()
}

pub fn formatar(dt: NaiveDateTime) -> String {
    dt.format(FORMATO).to_string()
}

pub fn formatar_data(d: NaiveDate) -> String {
    d.format(FORMATO_DATA).to_string()
}

pub fn analisar(texto: &str) -> Result<NaiveDateTime, ErroApp> {
    NaiveDateTime::parse_from_str(texto.trim(), FORMATO).map_err(|_| {
        ErroApp::validacao(format!(
            "Data-hora inválida: \"{texto}\". Use o formato AAAA-MM-DD HH:MM."
        ))
    })
}

pub fn analisar_data(texto: &str) -> Result<NaiveDate, ErroApp> {
    NaiveDate::parse_from_str(texto.trim(), FORMATO_DATA)
        .map_err(|_| ErroApp::validacao(format!("Data inválida: \"{texto}\". Use AAAA-MM-DD.")))
}

/// Uma data-hora está no futuro quando passa de `agora` por mais que a folga.
/// A folga existe porque o relógio da estação e o do servidor divergem alguns
/// segundos, e recusar por isso seria incompreensível para quem digita.
pub fn no_futuro_alem_de(dt: NaiveDateTime, minutos: i64) -> bool {
    dt > Local::now().naive_local() + Duration::minutes(minutos)
}

pub fn dias_atras(dt: NaiveDateTime) -> i64 {
    (Local::now().naive_local() - dt).num_days()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn analisa_e_formata_de_volta() {
        let dt = analisar("2026-09-19 22:40").expect("formato válido");
        assert_eq!(formatar(dt), "2026-09-19 22:40");
    }

    #[test]
    fn recusa_formato_com_segundos() {
        assert!(analisar("2026-09-19 22:40:15").is_err());
    }

    #[test]
    fn recusa_iso_com_t() {
        assert!(analisar("2026-09-19T22:40").is_err());
    }

    #[test]
    fn recusa_epoch() {
        assert!(analisar("1758320400").is_err());
    }
}
