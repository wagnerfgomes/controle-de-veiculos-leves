//! Formatação dos relatórios: HTML para ver e imprimir, CSV para o Excel.
//!
//! Funções puras sobre os dados já consultados. Nenhuma abre conexão, e é por
//! isso que dá para testar o CSV sem montar um banco.

use crate::db::relatorios::{LinhaRelatorio, RelatorioDados, TipoRelatorio};

/// O Excel em português só reconhece `;` como separador, e sem o BOM abre
/// "JOSÃ‰ CARLOS" em vez de "JOSÉ CARLOS". O sistema antigo já fazia assim.
pub const BOM_UTF8: &str = "\u{feff}";
const SEPARADOR: char = ';';

pub fn csv(dados: &RelatorioDados) -> String {
    let mut saida = String::from(BOM_UTF8);

    saida.push_str(&linha_csv(&["Relatório", dados.tipo.titulo()]));
    saida.push_str(&linha_csv(&[
        "Período",
        &dados.cabecalho.de,
        "a",
        &dados.cabecalho.ate,
    ]));
    saida.push_str(&linha_csv(&[
        "Viagens no período",
        &dados.cabecalho.viagens.to_string(),
    ]));
    saida.push_str(&linha_csv(&[
        "Viagens sem chegada",
        &dados.cabecalho.sem_chegada.to_string(),
        &formatar_pct(dados.cabecalho.sem_chegada_pct),
    ]));
    saida.push_str(&linha_csv(&[
        "Chegadas informadas manualmente",
        &dados.cabecalho.chegadas_manuais.to_string(),
        &formatar_pct(dados.cabecalho.chegadas_manuais_pct),
    ]));
    saida.push_str(&linha_csv(&[
        "Viagens com hodômetro nas duas pontas",
        &dados.cabecalho.hodometro_completo.to_string(),
        &formatar_pct(dados.cabecalho.hodometro_completo_pct),
    ]));
    saida.push_str(&linha_csv(&[
        "Emitido em",
        &dados.cabecalho.emitido_em,
        "por",
        &dados.cabecalho.emitido_por,
    ]));
    saida.push('\n');

    saida.push_str(&linha_csv(&colunas(dados.tipo)));
    for linha in &dados.linhas {
        saida.push_str(&linha_csv(&celulas(dados.tipo, linha)));
    }

    saida
}

fn colunas(tipo: TipoRelatorio) -> Vec<&'static str> {
    match tipo {
        TipoRelatorio::UsoVeiculo => vec![
            "Veículo",
            "Viagens",
            "Abertas",
            "Horas totais",
            "Horas média",
            "Chegadas manuais",
            "Km",
        ],
        TipoRelatorio::UsoMotorista => vec![
            "Condutor",
            "Viagens",
            "Abertas",
            "Horas totais",
            "Horas média",
            "Chegadas manuais",
            "Veículos distintos",
            "Frotas",
        ],
    }
}

fn celulas(tipo: TipoRelatorio, l: &LinhaRelatorio) -> Vec<String> {
    let mut celulas = vec![
        l.rotulo.clone(),
        l.viagens.to_string(),
        l.abertas.to_string(),
        numero(l.horas_totais),
        l.horas_media.map(numero).unwrap_or_default(),
        l.chegadas_manuais.to_string(),
    ];

    match tipo {
        TipoRelatorio::UsoVeiculo => {
            celulas.push(l.km.map(|k| k.to_string()).unwrap_or_default());
        }
        TipoRelatorio::UsoMotorista => {
            celulas.push(
                l.veiculos_distintos
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            );
            celulas.push(l.frotas.clone().unwrap_or_default());
        }
    }

    celulas
}

/// Vírgula decimal, que é o que o Excel em português espera.
fn numero(valor: f64) -> String {
    format!("{valor:.2}").replace('.', ",")
}

fn formatar_pct(valor: f64) -> String {
    format!("{}%", format!("{valor:.1}").replace('.', ","))
}

fn linha_csv<T: AsRef<str>>(celulas: &[T]) -> String {
    let mut linha = celulas
        .iter()
        .map(|c| escapar(c.as_ref()))
        .collect::<Vec<_>>()
        .join(&SEPARADOR.to_string());
    linha.push('\n');
    linha
}

fn escapar(valor: &str) -> String {
    if valor.contains(SEPARADOR) || valor.contains('"') || valor.contains('\n') {
        return format!("\"{}\"", valor.replace('"', "\"\""));
    }
    valor.to_string()
}

/// HTML autocontido: renderiza no próprio WebView e imprime em PDF por ele. Sem
/// CSS externo, sem fonte remota, sem script. As máquinas podem não ter internet.
pub fn html(dados: &RelatorioDados) -> String {
    let c = &dados.cabecalho;
    let mut corpo = String::new();

    corpo.push_str(&format!(
        "<h1>{}</h1>\n<p class=\"periodo\">{} a {} <span>(limite final exclusivo)</span></p>\n",
        escapar_html(dados.tipo.titulo()),
        escapar_html(&c.de),
        escapar_html(&c.ate)
    ));

    corpo.push_str("<table class=\"cabecalho\"><tbody>\n");
    corpo.push_str(&linha_cabecalho(
        "Viagens no período",
        &c.viagens.to_string(),
        None,
    ));
    corpo.push_str(&linha_cabecalho(
        "Viagens sem chegada",
        &c.sem_chegada.to_string(),
        Some(c.sem_chegada_pct),
    ));
    corpo.push_str(&linha_cabecalho(
        "Chegadas informadas manualmente",
        &c.chegadas_manuais.to_string(),
        Some(c.chegadas_manuais_pct),
    ));
    corpo.push_str(&linha_cabecalho(
        "Viagens com hodômetro nas duas pontas",
        &c.hodometro_completo.to_string(),
        Some(c.hodometro_completo_pct),
    ));
    corpo.push_str("</tbody></table>\n");

    corpo.push_str("<table class=\"dados\"><thead><tr>");
    for coluna in colunas(dados.tipo) {
        corpo.push_str(&format!("<th>{}</th>", escapar_html(coluna)));
    }
    corpo.push_str("</tr></thead><tbody>\n");

    for linha in &dados.linhas {
        corpo.push_str("<tr>");
        for (i, celula) in celulas(dados.tipo, linha).iter().enumerate() {
            let classe = if i == 0 { "" } else { " class=\"num\"" };
            corpo.push_str(&format!("<td{classe}>{}</td>", escapar_html(celula)));
        }
        corpo.push_str("</tr>\n");
    }

    if dados.linhas.is_empty() {
        corpo
            .push_str("<tr><td colspan=\"9\" class=\"vazio\">Nenhuma viagem no período.</td></tr>");
    }

    corpo.push_str("</tbody></table>\n");
    corpo.push_str(&format!(
        "<p class=\"rodape\">Emitido em {} por {}</p>\n",
        escapar_html(&c.emitido_em),
        escapar_html(&c.emitido_por)
    ));

    format!(
        "<!doctype html>\n<html lang=\"pt-BR\"><head><meta charset=\"utf-8\">\n\
         <title>{} — {} a {}</title>\n<style>{}</style></head>\n<body>\n{}</body></html>\n",
        escapar_html(dados.tipo.titulo()),
        escapar_html(&c.de),
        escapar_html(&c.ate),
        ESTILO,
        corpo
    )
}

fn linha_cabecalho(rotulo: &str, valor: &str, pct: Option<f64>) -> String {
    let complemento = match pct {
        Some(p) => format!(" <span class=\"pct\">({})</span>", formatar_pct(p)),
        None => String::new(),
    };
    format!(
        "<tr><th>{}</th><td>{}{}</td></tr>\n",
        escapar_html(rotulo),
        escapar_html(valor),
        complemento
    )
}

fn escapar_html(valor: &str) -> String {
    valor
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const ESTILO: &str = "
body { font-family: Arial, Helvetica, sans-serif; color: #1e2a3a; margin: 24px; }
h1 { color: #0f396d; font-size: 20px; margin: 0 0 4px; }
.periodo { margin: 0 0 16px; color: #6b7280; }
.periodo span { font-size: 12px; }
table { border-collapse: collapse; width: 100%; }
table.cabecalho { width: auto; margin-bottom: 20px; }
table.cabecalho th { text-align: left; padding: 3px 16px 3px 0; font-weight: 600; }
table.cabecalho td { padding: 3px 0; }
.pct { color: #6b7280; }
table.dados th { background: #0f396d; color: #fff; text-align: left; padding: 6px 8px;
                 font-size: 12px; }
table.dados td { border-bottom: 1px solid #d1d9e0; padding: 5px 8px; font-size: 13px; }
td.num { text-align: right; }
td.vazio { text-align: center; color: #6b7280; padding: 24px; }
.rodape { margin-top: 20px; color: #6b7280; font-size: 12px; }
@media print { body { margin: 0; } table.dados th { -webkit-print-color-adjust: exact; } }
";

#[cfg(test)]
mod testes {
    use super::*;
    use crate::db::relatorios::CabecalhoRelatorio;

    fn dados() -> RelatorioDados {
        RelatorioDados {
            tipo: TipoRelatorio::UsoVeiculo,
            cabecalho: CabecalhoRelatorio {
                de: "2026-09-01".to_string(),
                ate: "2026-10-01".to_string(),
                viagens: 10,
                sem_chegada: 2,
                sem_chegada_pct: 20.0,
                chegadas_manuais: 3,
                chegadas_manuais_pct: 30.0,
                hodometro_completo: 5,
                hodometro_completo_pct: 50.0,
                emitido_em: "2026-09-20 10:00".to_string(),
                emitido_por: "DOMINIO\\a.lopes".to_string(),
            },
            linhas: vec![LinhaRelatorio {
                rotulo: "907015 — Saveiro; cabine dupla".to_string(),
                viagens: 4,
                abertas: 1,
                horas_totais: 22.5,
                horas_media: Some(7.5),
                chegadas_manuais: 1,
                km: Some(340),
                veiculos_distintos: None,
                frotas: None,
            }],
        }
    }

    #[test]
    fn csv_comeca_com_bom_e_usa_ponto_e_virgula() {
        let saida = csv(&dados());
        assert!(saida.starts_with(BOM_UTF8), "o Excel precisa do BOM");
        assert!(saida.contains("Veículo;Viagens;Abertas"));
    }

    #[test]
    fn csv_usa_virgula_decimal() {
        let saida = csv(&dados());
        assert!(
            saida.contains("22,50"),
            "horas com vírgula decimal: {saida}"
        );
        assert!(!saida.contains("22.50"));
    }

    #[test]
    fn csv_escapa_celula_que_contem_o_separador() {
        let saida = csv(&dados());
        assert!(
            saida.contains("\"907015 — Saveiro; cabine dupla\""),
            "célula com ';' precisa de aspas: {saida}"
        );
    }

    #[test]
    fn csv_traz_o_cabecalho_obrigatorio_com_percentuais() {
        let saida = csv(&dados());
        assert!(saida.contains("Viagens sem chegada;2;20,0%"));
        assert!(saida.contains("Chegadas informadas manualmente;3;30,0%"));
        assert!(saida.contains("Viagens com hodômetro nas duas pontas;5;50,0%"));
        assert!(saida.contains("Emitido em;2026-09-20 10:00;por;"));
    }

    #[test]
    fn html_mostra_o_percentual_de_viagens_sem_chegada() {
        let saida = html(&dados());
        assert!(saida.contains("Viagens sem chegada"));
        assert!(saida.contains("20,0%"));
    }

    #[test]
    fn html_nao_busca_nada_na_rede() {
        let saida = html(&dados());
        for proibido in ["http://", "https://", "<script"] {
            assert!(
                !saida.contains(proibido),
                "relatório não pode depender de rede nem de script: {proibido}"
            );
        }
    }

    #[test]
    fn horas_media_ausente_vira_celula_vazia_e_nao_zero() {
        let mut d = dados();
        d.linhas[0].horas_media = None;
        let saida = csv(&d);
        assert!(
            saida.contains(";;1;340"),
            "média ausente fica vazia: {saida}"
        );
    }
}
