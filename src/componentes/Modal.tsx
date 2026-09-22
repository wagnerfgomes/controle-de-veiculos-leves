import type { ReactNode } from "react";
import { useEffect } from "react";

interface Props {
  titulo: string;
  largura?: number;
  onFechar: () => void;
  children: ReactNode;
  rodape?: ReactNode;
}

export function Modal({ titulo, largura = 560, onFechar, children, rodape }: Props) {
  useEffect(() => {
    const aoTeclar = (e: KeyboardEvent) => {
      if (e.key === "Escape") onFechar();
    };
    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, [onFechar]);

  return (
    <div className="fundo-modal" onMouseDown={onFechar}>
      <div
        className="cartao caixa-modal"
        style={{ width: largura }}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <header>
          <h2>{titulo}</h2>
          <button type="button" onClick={onFechar} aria-label="Fechar">
            ✕
          </button>
        </header>
        <div className="corpo-modal">{children}</div>
        {rodape && <footer>{rodape}</footer>}
      </div>
    </div>
  );
}
