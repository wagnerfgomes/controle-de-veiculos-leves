import { useState } from "react";

import { comoErroApp, exigeConfirmacao } from "../api/cliente";
import { encerrarSaida } from "../api/saidas";
import { Aviso } from "../componentes/Aviso";
import { EtiquetaTurno } from "../componentes/EtiquetaTurno";
import { Modal } from "../componentes/Modal";
import type { ErroApp, SaidaAberta } from "../tipos";

interface Props {
  saida: SaidaAberta;
  onFechar: () => void;
  onGravou: () => void;
}

/** O caminho rápido tem que ser de um clique: a alta demanda é o que produz as
 *  chegadas que ninguém registra. Informar outro horário fica atrás de um link. */
export function EncerrarSaida({ saida, onFechar, onGravou }: Props) {
  const [outroHorario, setOutroHorario] = useState(false);
  const [dtChegada, setDtChegada] = useState(agoraLocal());
  const [hodometro, setHodometro] = useState("");
  const [erro, setErro] = useState<ErroApp | null>(null);
  const [gravando, setGravando] = useState(false);

  const gravar = async (confirmadoAvisos: boolean) => {
    setGravando(true);
    try {
      await encerrarSaida(
        saida.id,
        outroHorario ? dtChegada : null,
        hodometro.trim() ? Number(hodometro) : null,
        confirmadoAvisos,
      );
      onGravou();
    } catch (bruto) {
      const e = comoErroApp(bruto);
      if (exigeConfirmacao(e) && window.confirm(`${e.mensagem}\n\nConfirma?`)) {
        setGravando(false);
        await gravar(true);
        return;
      }
      setErro(e);
    } finally {
      setGravando(false);
    }
  };

  return (
    <Modal
      titulo="Encerrar viagem"
      largura={460}
      onFechar={onFechar}
      rodape={
        <>
          <button type="button" onClick={onFechar}>
            Cancelar
          </button>
          <button
            type="button"
            className="verde grande"
            disabled={gravando}
            onClick={() => void gravar(false)}
          >
            {gravando ? "Encerrando…" : outroHorario ? "Encerrar" : "Encerrar agora"}
          </button>
        </>
      }
    >
      {erro && <Aviso erro={erro} onFechar={() => setErro(null)} />}

      <dl className="resumo">
        <dt>Veículo</dt>
        <dd>
          <strong>{saida.frota}</strong>
        </dd>
        <dt>Condutor</dt>
        <dd>{saida.motorista}</dd>
        <dt>Turno</dt>
        <dd>
          <EtiquetaTurno turno={saida.turno} />
        </dd>
        <dt>Saída</dt>
        <dd>
          {saida.dt_saida} <span className="sutil">({saida.horas_decorridas.toFixed(1)} h)</span>
        </dd>
        <dt>Destino</dt>
        <dd>{saida.destino ?? "—"}</dd>
        <dt>Atividade</dt>
        <dd>{saida.atividade}</dd>
      </dl>

      {outroHorario ? (
        <div className="campo">
          <label>Chegada</label>
          <input
            type="datetime-local"
            value={dtChegada.replace(" ", "T")}
            autoFocus
            onChange={(e) => setDtChegada(e.target.value.replace("T", " ").slice(0, 16))}
          />
        </div>
      ) : (
        <button type="button" className="link" onClick={() => setOutroHorario(true)}>
          informar outro horário
        </button>
      )}

      <div className="campo">
        <label>Hodômetro de chegada</label>
        <input
          type="number"
          value={hodometro}
          onChange={(e) => setHodometro(e.target.value)}
          placeholder="Opcional"
        />
      </div>
    </Modal>
  );
}

function agoraLocal(): string {
  const agora = new Date();
  const doisDigitos = (n: number) => String(n).padStart(2, "0");
  return (
    `${agora.getFullYear()}-${doisDigitos(agora.getMonth() + 1)}-${doisDigitos(agora.getDate())}` +
    ` ${doisDigitos(agora.getHours())}:${doisDigitos(agora.getMinutes())}`
  );
}
