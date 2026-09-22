//! Normalização de nome e de frota.
//!
//! `nome_norm` é gerado aqui, não em SQL, e é ele que carrega o `UNIQUE` do
//! schema. É a primeira barreira contra os 182 cadastros que o sistema antigo
//! acumulou para cerca de 70 pessoas.

use unicode_normalization::UnicodeNormalization;

/// `trim` → maiúsculas → decomposição NFD com descarte de marcas combinantes →
/// colapso de espaços múltiplos.
///
/// `"José  Carlos "` e `"JOSE CARLOS"` produzem a mesma chave.
pub fn nome_norm(bruto: &str) -> String {
    let sem_acento: String = bruto
        .trim()
        .to_uppercase()
        .nfd()
        .filter(|c| !marca_combinante(*c))
        .collect();

    sem_acento.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Mesmo tratamento, mais a remoção de todo espaço interno: frota é código, não
/// nome. `"907 015"` e `"907015"` são a mesma frota.
pub fn frota_norm(bruto: &str) -> String {
    nome_norm(bruto).split_whitespace().collect()
}

/// Faixa Unicode das marcas combinantes deixadas para trás pela decomposição NFD.
fn marca_combinante(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn jose_carlos_colide_com_jose_carlos() {
        assert_eq!(nome_norm("José  Carlos "), nome_norm("JOSE CARLOS"));
        assert_eq!(nome_norm("José  Carlos "), "JOSE CARLOS");
    }

    #[test]
    fn cedilha_e_til_somem() {
        assert_eq!(nome_norm("Conceição Assunção"), "CONCEICAO ASSUNCAO");
    }

    #[test]
    fn frota_perde_espaco_interno() {
        assert_eq!(frota_norm("907 015"), "907015");
        assert_eq!(frota_norm(" ab 1234 "), "AB1234");
    }

    #[test]
    fn nome_vazio_normaliza_para_vazio() {
        assert_eq!(nome_norm("   "), "");
    }
}
