import type { ErroApp } from "../tipos";

/** Mostra a mensagem que veio do Rust. A tela nunca reescreve o texto de erro:
 *  duas versões da mesma mensagem divergem e ninguém sabe qual é a verdadeira. */
export function Aviso({ erro, onFechar }: { erro: ErroApp; onFechar?: () => void }) {
  return (
    <div className={`aviso ${classe(erro)}`} role="alert">
      <span>{erro.mensagem}</span>
      {onFechar && (
        <button type="button" onClick={onFechar} aria-label="Fechar aviso">
          ✕
        </button>
      )}
    </div>
  );
}

function classe(erro: ErroApp): string {
  if (erro.codigo === "SEMELHANTE") return "atencao";
  if (erro.codigo === "VALIDACAO") return "atencao";
  return "erro";
}

export function Toast({ texto, onFechar }: { texto: string; onFechar: () => void }) {
  return (
    <div className="toast" role="status">
      <span>{texto}</span>
      <button type="button" onClick={onFechar} aria-label="Fechar">
        ✕
      </button>
    </div>
  );
}
