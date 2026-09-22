import { useCallback, useEffect, useState } from "react";

import { comoErroApp } from "../api/cliente";
import { listarMotoristas } from "../api/motoristas";
import { excluirSaida, listarSaidas } from "../api/saidas";
import { listarVeiculos } from "../api/veiculos";
import { Aviso, Toast } from "../componentes/Aviso";
import { EtiquetaTurno } from "../componentes/EtiquetaTurno";
import { FILTRO_VAZIO } from "../tipos";
import type { ErroApp, FiltroSaidas, Motorista, Saida, Turno, Veiculo } from "../tipos";

export function Historico() {
  const [filtro, setFiltro] = useState<FiltroSaidas>(FILTRO_VAZIO);
  const [saidas, setSaidas] = useState<Saida[]>([]);
  const [veiculos, setVeiculos] = useState<Veiculo[]>([]);
  const [motoristas, setMotoristas] = useState<Motorista[]>([]);
  const [erro, setErro] = useState<ErroApp | null>(null);
  const [aviso, setAviso] = useState<string | null>(null);

  const recarregar = useCallback(() => {
    listarSaidas(filtro)
      .then((s) => {
        setSaidas(s);
        setErro(null);
      })
      .catch((e) => setErro(comoErroApp(e)));
  }, [filtro]);

  useEffect(recarregar, [recarregar]);

  useEffect(() => {
    Promise.all([listarVeiculos(true), listarMotoristas(true)])
      .then(([v, m]) => {
        setVeiculos(v);
        setMotoristas(m);
      })
      .catch((e) => setErro(comoErroApp(e)));
  }, []);

  const excluir = async (saida: Saida) => {
    // O motivo é obrigatório no Rust: exclusão sem motivo é a que ninguém
    // consegue explicar três meses depois.
    const motivo = window.prompt(
      `Excluir a viagem de ${saida.frota} em ${saida.dt_saida}?\n\nMotivo (obrigatório):`,
    );
    if (motivo === null) return;

    try {
      await excluirSaida(saida.id, motivo);
      setAviso("Viagem excluída. O veículo voltou a ficar disponível.");
      recarregar();
    } catch (bruto) {
      setErro(comoErroApp(bruto));
    }
  };

  const alternarTurno = (turno: Turno) =>
    setFiltro((f) => ({
      ...f,
      turnos: f.turnos.includes(turno)
        ? f.turnos.filter((t) => t !== turno)
        : [...f.turnos, turno],
    }));

  return (
    <div className="historico">
      {erro && <Aviso erro={erro} onFechar={() => setErro(null)} />}
      {aviso && <Toast texto={aviso} onFechar={() => setAviso(null)} />}

      <section className="cartao bloco filtros">
        <div className="campo">
          <label>De</label>
          <input
            type="date"
            value={filtro.de ?? ""}
            onChange={(e) => setFiltro({ ...filtro, de: e.target.value || null })}
          />
        </div>
        <div className="campo">
          <label>
            Até <span className="sutil">(exclusivo)</span>
          </label>
          <input
            type="date"
            value={filtro.ate ?? ""}
            onChange={(e) => setFiltro({ ...filtro, ate: e.target.value || null })}
          />
        </div>
        <div className="campo">
          <label>Veículo</label>
          <select
            value={filtro.veiculo_id ?? ""}
            onChange={(e) =>
              setFiltro({ ...filtro, veiculo_id: e.target.value ? Number(e.target.value) : null })
            }
          >
            <option value="">Todos</option>
            {veiculos.map((v) => (
              <option key={v.id} value={v.id}>
                {v.frota}
              </option>
            ))}
          </select>
        </div>
        <div className="campo">
          <label>Condutor</label>
          <select
            value={filtro.motorista_id ?? ""}
            onChange={(e) =>
              setFiltro({
                ...filtro,
                motorista_id: e.target.value ? Number(e.target.value) : null,
              })
            }
          >
            <option value="">Todos</option>
            {motoristas.map((m) => (
              <option key={m.id} value={m.id}>
                {m.nome}
              </option>
            ))}
          </select>
        </div>
        <div className="campo">
          <label>Turno</label>
          <div className="botoes-turno">
            {(["A", "B", "C"] as const).map((t) => (
              <button
                key={t}
                type="button"
                className={filtro.turnos.includes(t) ? `turno-escolhido ${t}` : ""}
                onClick={() => alternarTurno(t)}
              >
                {t}
              </button>
            ))}
          </div>
        </div>
        <div className="campo caixas">
          <label>
            <input
              type="checkbox"
              checked={filtro.somente_abertas}
              onChange={(e) => setFiltro({ ...filtro, somente_abertas: e.target.checked })}
            />{" "}
            Só abertas
          </label>
          <label>
            <input
              type="checkbox"
              checked={filtro.incluir_excluidas}
              onChange={(e) => setFiltro({ ...filtro, incluir_excluidas: e.target.checked })}
            />{" "}
            Mostrar excluídas
          </label>
        </div>
        <button type="button" onClick={() => setFiltro(FILTRO_VAZIO)}>
          Limpar
        </button>
      </section>

      <section className="cartao bloco">
        <h2>
          Viagens <span className="sutil">({saidas.length})</span>
        </h2>
        <div className="rolagem">
          <table>
            <thead>
              <tr>
                <th>Frota</th>
                <th>Condutor</th>
                <th>Turno</th>
                <th>Saída</th>
                <th>Chegada</th>
                <th>Destino</th>
                <th>Atividade</th>
                <th className="num">Km</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {saidas.map((s) => (
                <tr key={s.id} className={s.excluida_em ? "excluida" : ""}>
                  <td>
                    <strong>{s.frota}</strong>
                  </td>
                  <td>{s.motorista}</td>
                  <td>
                    <EtiquetaTurno turno={s.turno} />
                  </td>
                  <td>{s.dt_saida}</td>
                  <td>
                    {s.dt_chegada ?? <span className="sutil">na rua</span>}
                    {s.chegada_manual && <span className="sutil" title="Informada à mão"> ✎</span>}
                  </td>
                  <td>{s.destino ?? "—"}</td>
                  <td>
                    {s.atividade}
                    {s.excluida_motivo && (
                      <div className="sutil">excluída: {s.excluida_motivo}</div>
                    )}
                  </td>
                  <td className="num">{km(s)}</td>
                  <td className="num">
                    {!s.excluida_em && (
                      <button type="button" className="perigo" onClick={() => void excluir(s)}>
                        Excluir
                      </button>
                    )}
                  </td>
                </tr>
              ))}
              {saidas.length === 0 && (
                <tr>
                  <td colSpan={9} className="sutil vazio">
                    Nenhuma viagem com esses filtros.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}

function km(s: Saida): string {
  if (s.hodometro_saida == null || s.hodometro_chegada == null) return "—";
  return String(s.hodometro_chegada - s.hodometro_saida);
}
