//! Detecção de cadastro parecido, a rede de segurança do cadastro rápido.
//!
//! Distância igual a zero não é semelhança, é duplicata: sai por `DUPLICADO`,
//! pelo `UNIQUE` do schema, antes de chegar aqui.

use strsim::levenshtein;

pub const DISTANCIA_NOME: usize = 2;
pub const DISTANCIA_FROTA: usize = 1;

pub fn distancia(a: &str, b: &str) -> usize {
    levenshtein(a, b)
}

/// Ambos os lados já devem estar normalizados por [`super::nome::nome_norm`].
pub fn nomes_semelhantes(a: &str, b: &str) -> bool {
    let d = distancia(a, b);
    d > 0 && d <= DISTANCIA_NOME
}

/// Ambos os lados já devem estar normalizados por [`super::nome::frota_norm`].
pub fn frotas_semelhantes(a: &str, b: &str) -> bool {
    let d = distancia(a, b);
    d > 0 && d <= DISTANCIA_FROTA
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::dominio::nome::{frota_norm, nome_norm};

    #[test]
    fn angelo_junior_alerta_contra_anjelo_junior() {
        let a = nome_norm("ANGELO JUNIOR");
        let b = nome_norm("ANJELO JUNIOR");
        assert!(nomes_semelhantes(&a, &b));
    }

    #[test]
    fn nome_identico_nao_e_semelhante_e_sim_duplicata() {
        let a = nome_norm("José Carlos");
        let b = nome_norm("JOSE  CARLOS");
        assert!(!nomes_semelhantes(&a, &b));
    }

    #[test]
    fn nomes_distintos_nao_alertam() {
        let a = nome_norm("ANGELO JUNIOR");
        let b = nome_norm("MARIA DAS GRACAS");
        assert!(!nomes_semelhantes(&a, &b));
    }

    #[test]
    fn frota_907218_alerta_contra_907018() {
        let a = frota_norm("907218");
        let b = frota_norm("907018");
        assert!(frotas_semelhantes(&a, &b));
    }

    #[test]
    fn frota_com_dois_digitos_de_diferenca_nao_alerta() {
        assert!(!frotas_semelhantes(
            &frota_norm("907218"),
            &frota_norm("907011")
        ));
    }
}
