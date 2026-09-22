import { useState } from "react";

import { comoErroApp } from "../api/cliente";
import { gerarRelatorio, previaRelatorio } from "../api/relatorios";
import { Aviso, Toast } from "../componentes/Aviso";
import { FILTRO_VAZIO } from "../tipos";
import type { ErroApp, Formato, RelatorioDados, TipoRelatorio } from "../tipos";

const TIPOS: { chave: TipoRelatorio; rotulo: string }[] = [
  { chave: "UsoVeiculo", rotulo: "Uso de veículo" },
  { chave: "UsoMotorista", rotulo: "Uso por condutor" },
];

/** Emissão manual. A varredura automática de meses pendentes é da Fase 9. */
export function Relatorios() {
  const [tipo, setTipo] = useState<TipoRelatorio>("UsoVeiculo");
  const [de, setDe] = useState(primeiroDoMes());
  const [ate, setAte] = useState(primeiroDoMesSeguinte());
  const [dados, setDados] = useState<RelatorioDados | null>(null);
  const [erro, setErro] = useState<ErroApp | null>(null);
  const [aviso, setAviso] = useState<string | null>(null);

  const filtro = { ...FILTRO_VAZIO, de, ate };

  const carregarPrevia = () =>
    previaRelatorio(tipo, filtro)
      .then((d) => {
        setDados(d);
        setErro(null);
      })
      .catch((e) => setErro(comoErroApp(e)));

  const gerar = (formato: Formato) =>
    gerarRelatorio(tipo, filtro, formato)
      .then((caminho) => setAviso(`Relatório gravado em ${caminho}`))
      .catch((e) => setErro(comoErroApp(e)));

  const c = dados?.cabecalho;

  return (
    <div className="relatorios">
      {erro && <Aviso erro={erro} onFechar={() => setErro(null)} />}
      {aviso && <Toast texto={aviso} onFechar={() => setAviso(null)} />}

      <section className="cartao bloco filtros">
        <div className="campo">
          <label>Relatório</label>
          <select value={tipo} onChange={(e) => setTipo(e.target.value as TipoRelatorio)}>
            {TIPOS.map((t) => (
              <option key={t.chave} value={t.chave}>
                {t.rotulo}
              </option>
            ))}
          </select>
        </div>
        <div className="campo">
          <label>De</label>
          <input type="date" value={de} onChange={(e) => setDe(e.target.value)} />
        </div>
        <div className="campo">
          <label>
            Até <span className="sutil">(exclusivo)</span>
          </label>
          <input type="date" value={ate} onChange={(e) => setAte(e.target.value)} />
        </div>
        <button type="button" className="primario" onClick={() => void carregarPrevia()}>
          Ver prévia
        </button>
        <button type="button" onClick={() => void gerar("Csv")}>
          Gerar CSV
        </button>
        <button type="button" onClick={() => void gerar("Html")}>
          Gerar HTML
        </button>
      </section>

      {c && (
        <section className="cartao bloco">
          <h2>Cabeçalho do período</h2>
          <dl className="resumo cabecalho-relatorio">
            <dt>Período</dt>
            <dd>
              {c.de} a {c.ate}
            </dd>
            <dt>Viagens no período</dt>
            <dd>{c.viagens}</dd>
            <dt>Viagens sem chegada</dt>
            <dd>
              {c.sem_chegada} <span className="sutil">({c.sem_chegada_pct}%)</span>
            </dd>
            <dt>Chegadas informadas manualmente</dt>
            <dd>
              {c.chegadas_manuais} <span className="sutil">({c.chegadas_manuais_pct}%)</span>
            </dd>
            <dt>Hodômetro nas duas pontas</dt>
            <dd>
              {c.hodometro_completo} <span className="sutil">({c.hodometro_completo_pct}%)</span>
            </dd>
            <dt>Emitido em</dt>
            <dd>
              {c.emitido_em} por {c.emitido_por}
            </dd>
          </dl>
        </section>
      )}

      {dados && (
        <section className="cartao bloco">
          <h2>{TIPOS.find((t) => t.chave === dados.tipo)?.rotulo}</h2>
          <div className="rolagem">
            <table>
              <thead>
                <tr>
                  <th>{dados.tipo === "UsoVeiculo" ? "Veículo" : "Condutor"}</th>
                  <th className="num">Viagens</th>
                  <th className="num">Abertas</th>
                  <th className="num">Horas totais</th>
                  <th className="num">Horas média</th>
                  <th className="num">Chegadas manuais</th>
                  {dados.tipo === "UsoVeiculo" ? (
                    <th className="num">Km</th>
                  ) : (
                    <>
                      <th className="num">Veículos</th>
                      <th>Frotas</th>
                    </>
                  )}
                </tr>
              </thead>
              <tbody>
                {dados.linhas.map((l) => (
                  <tr key={l.rotulo}>
                    <td>
                      <strong>{l.rotulo}</strong>
                    </td>
                    <td className="num">{l.viagens}</td>
                    <td className="num">{l.abertas}</td>
                    <td className="num">{l.horas_totais.toFixed(2)}</td>
                    {/* Média ausente é "—": zero afirmaria que rodou sem gastar tempo. */}
                    <td className="num">{l.horas_media?.toFixed(2) ?? "—"}</td>
                    <td className="num">{l.chegadas_manuais}</td>
                    {dados.tipo === "UsoVeiculo" ? (
                      <td className="num">{l.km ?? "—"}</td>
                    ) : (
                      <>
                        <td className="num">{l.veiculos_distintos ?? "—"}</td>
                        <td>{l.frotas ?? "—"}</td>
                      </>
                    )}
                  </tr>
                ))}
                {dados.linhas.length === 0 && (
                  <tr>
                    <td colSpan={8} className="sutil vazio">
                      Nenhuma viagem no período.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </section>
      )}
    </div>
  );
}

function primeiroDoMes(): string {
  const hoje = new Date();
  return `${hoje.getFullYear()}-${String(hoje.getMonth() + 1).padStart(2, "0")}-01`;
}

/** O limite é exclusivo: "setembro inteiro" é de 2026-09-01 a 2026-10-01. */
function primeiroDoMesSeguinte(): string {
  const hoje = new Date();
  const seguinte = new Date(hoje.getFullYear(), hoje.getMonth() + 1, 1);
  return `${seguinte.getFullYear()}-${String(seguinte.getMonth() + 1).padStart(2, "0")}-01`;
}
