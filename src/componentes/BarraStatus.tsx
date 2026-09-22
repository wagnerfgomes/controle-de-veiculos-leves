import { useState } from "react";

import { comoErroApp } from "../api/cliente";
import { estadoSessao, forcarBackup } from "../api/sessao";
import type { EstadoSessao } from "../tipos";

interface Props {
  sessao: EstadoSessao;
  aoAtualizar: (sessao: EstadoSessao) => void;
}

export function BarraStatus({ sessao, aoAtualizar }: Props) {
  const [mensagem, setMensagem] = useState<string | null>(null);

  const backup = async () => {
    setMensagem("Gerando backup…");
    try {
      const caminho = await forcarBackup();
      setMensagem(`Backup em ${caminho}`);
      aoAtualizar(await estadoSessao());
    } catch (bruto) {
      setMensagem(comoErroApp(bruto).mensagem);
    }
  };

  return (
    <footer className="barra-status">
      <span>{sessao.usuario_windows}</span>
      <span className="separador">·</span>
      <span>{sessao.maquina}</span>
      <span className="separador">·</span>
      <span className={sessao.conexao === "Normal" ? "conexao-ok" : "conexao-ruim"}>
        {sessao.conexao === "Normal" ? "servidor conectado" : "conexão perdida"}
      </span>
      <span className="separador">·</span>
      <span title={sessao.caminho_dados}>{sessao.caminho_dados}</span>
      <span className="separador">·</span>
      <span>último backup: {sessao.ultimo_backup ?? "—"}</span>

      {/* Em Linux o lock é um stub de flock. A barra é obrigada a dizer isso:
          stub silencioso é como se descobre em produção que nunca houve lock. */}
      {!sessao.lock_confiavel && (
        <span className="lock-stub" title="flock advisory, não protege contra outra máquina">
          LOCK DE DESENVOLVIMENTO
        </span>
      )}

      <span className="espaco" />

      {mensagem && <span className="sutil">{mensagem}</span>}

      {sessao.modo === "producao" && (
        <button type="button" onClick={() => void backup()}>
          Backup agora
        </button>
      )}
    </footer>
  );
}
