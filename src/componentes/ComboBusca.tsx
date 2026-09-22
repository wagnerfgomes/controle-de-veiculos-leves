import { useEffect, useRef, useState } from "react";

export interface ItemCombo {
  id: number;
  rotulo: string;
  /** Segunda linha: modelo do carro, matrícula do condutor, quem está com ele. */
  detalhe?: string | null;
  indisponivel?: boolean;
}

interface Props {
  rotulo: string;
  itens: ItemCombo[];
  valor: number | null;
  onEscolher: (id: number | null) => void;
  /** Texto digitado que não casou com nada, para o "Cadastrar 907015". */
  onCadastrar?: (termo: string) => void;
  autoFocus?: boolean;
  placeholder?: string;
}

/** Combo com busca. Os indisponíveis aparecem esmaecidos em vez de sumir: quem
 *  registra precisa saber a quem recorrer, não só que o carro não dá. */
export function ComboBusca({
  rotulo,
  itens,
  valor,
  onEscolher,
  onCadastrar,
  autoFocus,
  placeholder,
}: Props) {
  const [termo, setTermo] = useState("");
  const [aberto, setAberto] = useState(false);
  const caixa = useRef<HTMLDivElement>(null);

  const escolhido = itens.find((i) => i.id === valor) ?? null;

  useEffect(() => {
    const aoClicarFora = (e: MouseEvent) => {
      if (caixa.current && !caixa.current.contains(e.target as Node)) setAberto(false);
    };
    document.addEventListener("mousedown", aoClicarFora);
    return () => document.removeEventListener("mousedown", aoClicarFora);
  }, []);

  const normalizar = (texto: string) =>
    texto
      .normalize("NFD")
      .replace(/[̀-ͯ]/g, "")
      .toUpperCase();

  const alvo = normalizar(termo.trim());
  const filtrados = alvo
    ? itens.filter(
        (i) =>
          normalizar(i.rotulo).includes(alvo) ||
          normalizar(i.detalhe ?? "").includes(alvo),
      )
    : itens;

  const nadaEncontrado = alvo.length > 0 && filtrados.length === 0;

  return (
    <div className="campo" ref={caixa}>
      <label>{rotulo}</label>
      <div className="combo">
        <input
          value={aberto ? termo : (escolhido?.rotulo ?? "")}
          placeholder={placeholder ?? "Digite para buscar"}
          autoFocus={autoFocus}
          onFocus={() => {
            setAberto(true);
            setTermo("");
          }}
          onChange={(e) => {
            setTermo(e.target.value);
            setAberto(true);
          }}
        />
        {escolhido && !aberto && (
          <button
            type="button"
            className="limpar"
            onClick={() => onEscolher(null)}
            aria-label="Limpar"
          >
            ✕
          </button>
        )}

        {aberto && (
          <ul className="lista-combo">
            {filtrados.map((item) => (
              <li key={item.id}>
                <button
                  type="button"
                  className={item.indisponivel ? "indisponivel" : ""}
                  onClick={() => {
                    onEscolher(item.id);
                    setAberto(false);
                  }}
                >
                  <strong>{item.rotulo}</strong>
                  {item.detalhe && <small>{item.detalhe}</small>}
                </button>
              </li>
            ))}

            {nadaEncontrado && (
              <li className="vazio">
                {onCadastrar ? (
                  <button
                    type="button"
                    className="cadastrar"
                    onClick={() => {
                      onCadastrar(termo.trim());
                      setAberto(false);
                    }}
                  >
                    Cadastrar {termo.trim()}
                  </button>
                ) : (
                  <span className="sutil">Nada encontrado.</span>
                )}
              </li>
            )}
          </ul>
        )}
      </div>
    </div>
  );
}
