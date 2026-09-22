import { useCallback, useEffect, useState } from "react";

import { kpis as buscarKpis, listarSaidasAbertas } from "../api/saidas";
import { listarVeiculosDisponiveis } from "../api/veiculos";
import { comoErroApp } from "../api/cliente";
import { Aviso } from "../componentes/Aviso";
import { EtiquetaTurno } from "../componentes/EtiquetaTurno";
import { EncerrarSaida } from "./EncerrarSaida";
import { NovaSaida } from "./NovaSaida";
import { FILTRO_VAZIO } from "../tipos";
import type {
  ErroApp,
  Kpis,
  SaidaAberta,
  VeiculoDisponibilidade,
} from "../tipos";

/** Responde à pergunta que o setor faz o tempo todo: quais carros estão na rua
 *  e há quanto tempo. */
export function Painel() {
  const [abertas, setAbertas] = useState<SaidaAberta[]>([]);
  const [veiculos, setVeiculos] = useState<VeiculoDisponibilidade[]>([]);
  const [indicadores, setIndicadores] = useState<Kpis | null>(null);
  const [erro, setErro] = useState<ErroApp | null>(null);
  const [novaAberta, setNovaAberta] = useState(false);
  const [encerrando, setEncerrando] = useState<SaidaAberta | null>(null);

  const recarregar = useCallback(() => {
    Promise.all([
      listarSaidasAbertas(),
      listarVeiculosDisponiveis(),
      buscarKpis(FILTRO_VAZIO),
    ])
      .then(([a, v, k]) => {
        setAbertas(a);
        setVeiculos(v);
        setIndicadores(k);
        setErro(null);
      })
      .catch((e) => setErro(comoErroApp(e)));
  }, []);

  useEffect(recarregar, [recarregar]);

  // O tempo decorrido é calculado no Rust. Recarregar de minuto em minuto mantém
  // a coluna viva sem duplicar a regra das 24 h aqui na tela.
  useEffect(() => {
    const relogio = window.setInterval(recarregar, 60_000);
    return () => window.clearInterval(relogio);
  }, [recarregar]);

  const disponiveis = veiculos.filter((v) => v.disponivel);

  return (
    <div className="painel">
      {erro && <Aviso erro={erro} onFechar={() => setErro(null)} />}

      <section className="kpis">
        <Indicador rotulo="Viagens" valor={indicadores?.viagens ?? 0} />
        <Indicador rotulo="Na rua agora" valor={abertas.length} destaque />
        <Indicador rotulo="Condutores" valor={indicadores?.condutores_distintos ?? 0} />
        <Indicador rotulo="Frotas" valor={indicadores?.frotas_distintas ?? 0} />
        <Indicador
          rotulo="Duração média"
          // `null` é ausência de dado: nenhuma viagem fechou ainda. Mostrar
          // "0 h" afirmaria que os carros rodaram sem gastar tempo.
          texto={
            indicadores?.duracao_media_horas == null
              ? "—"
              : `${indicadores.duracao_media_horas.toFixed(2)} h`
          }
        />
        <div className="cartao indicador turnos">
          <span className="rotulo">Por turno</span>
          <div className="turno-linha">
            {(["A", "B", "C"] as const).map((t, i) => (
              <span key={t}>
                <EtiquetaTurno turno={t} /> {indicadores?.por_turno[i] ?? 0}
              </span>
            ))}
          </div>
        </div>
      </section>

      <div className="acoes-painel">
        <button
          type="button"
          className="primario grande"
          onClick={() => setNovaAberta(true)}
        >
          + Nova saída
        </button>
      </div>

      <section className="cartao bloco">
        <h2>Viagens abertas</h2>
        {abertas.length === 0 ? (
          <p className="sutil vazio">Nenhum veículo na rua no momento.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Frota</th>
                <th>Condutor</th>
                <th>Turno</th>
                <th>Saída</th>
                <th>Destino</th>
                <th>Atividade</th>
                <th className="num">Há</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {abertas.map((s) => (
                <tr key={s.id} className={s.alerta ? "alerta" : ""}>
                  <td>
                    <strong>{s.frota}</strong>
                  </td>
                  <td>{s.motorista}</td>
                  <td>
                    <EtiquetaTurno turno={s.turno} />
                  </td>
                  <td>{s.dt_saida}</td>
                  <td>{s.destino ?? "—"}</td>
                  <td>{s.atividade}</td>
                  <td className="num">{s.horas_decorridas.toFixed(1)} h</td>
                  <td className="num">
                    <button type="button" className="verde" onClick={() => setEncerrando(s)}>
                      Encerrar
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="cartao bloco">
        <h2>
          Veículos disponíveis <span className="sutil">({disponiveis.length})</span>
        </h2>
        {veiculos.length === 0 ? (
          <p className="sutil vazio">
            Nenhum veículo cadastrado. Comece pela aba Gestão.
          </p>
        ) : (
          <div className="grade-veiculos">
            {veiculos.map((v) => (
              <div
                key={v.veiculo.id}
                className={`chip-veiculo ${v.disponivel ? "" : "ocupado"}`}
                title={
                  v.disponivel
                    ? undefined
                    : `Com ${v.motorista_atual} desde ${v.desde}`
                }
              >
                <strong>{v.veiculo.frota}</strong>
                <small>{v.veiculo.modelo ?? "—"}</small>
                {!v.disponivel && <small className="com-quem">{v.motorista_atual}</small>}
              </div>
            ))}
          </div>
        )}
      </section>

      {novaAberta && (
        <NovaSaida
          onFechar={() => setNovaAberta(false)}
          onGravou={() => {
            setNovaAberta(false);
            recarregar();
          }}
        />
      )}

      {encerrando && (
        <EncerrarSaida
          saida={encerrando}
          onFechar={() => setEncerrando(null)}
          onGravou={() => {
            setEncerrando(null);
            recarregar();
          }}
        />
      )}
    </div>
  );
}

function Indicador({
  rotulo,
  valor,
  texto,
  destaque,
}: {
  rotulo: string;
  valor?: number;
  texto?: string;
  destaque?: boolean;
}) {
  return (
    <div className={`cartao indicador ${destaque ? "destaque" : ""}`}>
      <span className="rotulo">{rotulo}</span>
      <span className="valor">{texto ?? valor}</span>
    </div>
  );
}
