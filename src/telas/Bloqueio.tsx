import { useEffect, useState } from "react";

import { comoErroApp } from "../api/cliente";
import { infoBloqueio, tomarPosseLock } from "../api/sessao";
import { Aviso } from "../componentes/Aviso";
import type { ErroApp, EstadoSessao, InfoBloqueio } from "../tipos";

interface Props {
  aoEntrar: (sessao: EstadoSessao) => void;
  aoTentarNovamente: () => void;
}

/** Sem menu e sem contorno. O caminho certo é procurar a pessoa, não insistir
 *  no botão: a tomada de posse só aparece com o heartbeat parado há mais de
 *  10 minutos, e ainda assim exige digitar CONFIRMAR. */
export function Bloqueio({ aoEntrar, aoTentarNovamente }: Props) {
  const [info, setInfo] = useState<InfoBloqueio | null>(null);
  const [erro, setErro] = useState<ErroApp | null>(null);
  const [confirmacao, setConfirmacao] = useState("");
  const [tomando, setTomando] = useState(false);

  useEffect(() => {
    infoBloqueio()
      .then(setInfo)
      .catch((e) => setErro(comoErroApp(e)));
  }, []);

  const tomarPosse = async () => {
    setTomando(true);
    try {
      const sessao = await tomarPosseLock(confirmacao);
      aoEntrar(sessao);
    } catch (bruto) {
      setErro(comoErroApp(bruto));
    } finally {
      setTomando(false);
    }
  };

  return (
    <div className="tela-bloqueio">
      <div className="cartao">
        <h1>Sistema em uso</h1>

        {info ? (
          <p className="quem">
            {info.usuario_windows} · {info.maquina} · desde {info.desde}
          </p>
        ) : (
          <p className="quem sutil">Não foi possível identificar quem está usando.</p>
        )}

        <p>Procure essa pessoa antes de tentar novamente.</p>

        {erro && <Aviso erro={erro} onFechar={() => setErro(null)} />}

        <button type="button" className="primario grande" onClick={aoTentarNovamente}>
          Tentar novamente
        </button>

        {info?.pode_tomar_posse && (
          <div className="tomada-posse">
            <p className="sutil">
              A outra sessão está sem sinal há {info.minutos_sem_heartbeat} minutos e
              provavelmente foi encerrada de forma abrupta.
            </p>
            <p className="alerta-forte">
              Assumir a sessão enquanto o outro computador ainda estiver gravando pode
              corromper o banco. Só faça isso se tiver certeza de que aquele sistema está
              fechado.
            </p>
            <label>
              Digite CONFIRMAR para assumir
              <input
                value={confirmacao}
                onChange={(e) => setConfirmacao(e.target.value)}
                placeholder="CONFIRMAR"
              />
            </label>
            <button
              type="button"
              className="perigo"
              disabled={confirmacao.trim() !== "CONFIRMAR" || tomando}
              onClick={() => void tomarPosse()}
            >
              {tomando ? "Assumindo…" : "Assumir a sessão"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
