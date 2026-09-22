import { useCallback, useEffect, useState } from "react";

import { iniciarSessao } from "./api/sessao";
import { comoErroApp, partesDoDetalhe } from "./api/cliente";
import { Bloqueio } from "./telas/Bloqueio";
import { Gestao } from "./telas/Gestao";
import { Historico } from "./telas/Historico";
import { Painel } from "./telas/Painel";
import { Relatorios } from "./telas/Relatorios";
import { BarraStatus } from "./componentes/BarraStatus";
import type { ErroApp, EstadoSessao } from "./tipos";

type Aba = "painel" | "historico" | "gestao" | "relatorios";

const ABAS: { chave: Aba; rotulo: string }[] = [
  { chave: "painel", rotulo: "Painel" },
  { chave: "historico", rotulo: "Histórico" },
  { chave: "gestao", rotulo: "Gestão" },
  { chave: "relatorios", rotulo: "Relatórios" },
];

export function App() {
  const [sessao, setSessao] = useState<EstadoSessao | null>(null);
  const [erro, setErro] = useState<ErroApp | null>(null);
  const [aba, setAba] = useState<Aba>("painel");

  const abrir = useCallback(() => {
    setErro(null);
    iniciarSessao()
      .then(setSessao)
      .catch((e) => setErro(comoErroApp(e)));
  }, []);

  useEffect(abrir, [abrir]);

  // O lock tomado não é erro de sistema: é outra pessoa usando, e a tela precisa
  // dizer quem, para o caminho certo ser procurá-la em vez de insistir no botão.
  const emUso =
    erro !== null && partesDoDetalhe(erro).some((p) => p.prefixo === "EM_USO");

  if (emUso) return <Bloqueio aoEntrar={setSessao} aoTentarNovamente={abrir} />;

  if (erro) {
    return (
      <div className="tela-erro">
        <div className="cartao">
          <h1>Não foi possível abrir o sistema</h1>
          <p>{erro.mensagem}</p>
          {erro.detalhe && <p className="sutil">{erro.detalhe}</p>}
          <button type="button" className="primario grande" onClick={abrir}>
            Tentar novamente
          </button>
        </div>
      </div>
    );
  }

  if (!sessao) {
    return (
      <div className="tela-erro">
        <p className="sutil">Abrindo o sistema…</p>
      </div>
    );
  }

  return (
    <div className="aplicacao">
      {sessao.modo === "demonstracao" && (
        <div className="faixa-demo">
          MODO DEMONSTRAÇÃO · dados fictícios · tudo é apagado ao reabrir
        </div>
      )}

      <header className="cabecalho">
        <h1>Controle de Veículos Leves</h1>
        <nav>
          {ABAS.map(({ chave, rotulo }) => (
            <button
              key={chave}
              type="button"
              className={aba === chave ? "ativa" : ""}
              onClick={() => setAba(chave)}
            >
              {rotulo}
            </button>
          ))}
        </nav>
      </header>

      <main className="conteudo">
        {aba === "painel" && <Painel />}
        {aba === "historico" && <Historico />}
        {aba === "gestao" && <Gestao />}
        {aba === "relatorios" && <Relatorios />}
      </main>

      <BarraStatus sessao={sessao} aoAtualizar={setSessao} />
    </div>
  );
}
