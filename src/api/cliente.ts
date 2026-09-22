import { invoke } from "@tauri-apps/api/core";

import type { ErroApp } from "../tipos";

/** Todo erro que vem do Rust tem `codigo`. O React trata pelo código, nunca pela
 *  mensagem: mensagem é para humano e muda, código é contrato. */
export function ehErroApp(valor: unknown): valor is ErroApp {
  return (
    typeof valor === "object" &&
    valor !== null &&
    "codigo" in valor &&
    typeof (valor as ErroApp).codigo === "string"
  );
}

export function comoErroApp(valor: unknown): ErroApp {
  if (ehErroApp(valor)) return valor;
  return {
    codigo: "BANCO",
    mensagem: "Falha inesperada na comunicação com o sistema.",
    detalhe: String(valor),
  };
}

export async function chamar<T>(
  comando: string,
  argumentos?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(comando, argumentos);
  } catch (erro) {
    throw comoErroApp(erro);
  }
}

/** Quebra o `detalhe` na forma `PREFIXO:resto`, que é como o Rust marca o motivo
 *  estável de um `VALIDACAO` ou `SEMELHANTE`. */
export function partesDoDetalhe(erro: ErroApp): { prefixo: string; resto: string }[] {
  if (!erro.detalhe) return [];
  return erro.detalhe.split(";").map((parte) => {
    const corte = parte.indexOf(":");
    if (corte < 0) return { prefixo: parte, resto: "" };
    return { prefixo: parte.slice(0, corte), resto: parte.slice(corte + 1) };
  });
}

export function exigeConfirmacao(erro: ErroApp): boolean {
  if (erro.codigo !== "VALIDACAO") return false;
  return partesDoDetalhe(erro).some((p) =>
    ["DURACAO_LONGA", "SAIDA_ANTIGA", "HODOMETRO_MENOR", "HODOMETRO_DIFERENCA"].includes(
      p.prefixo,
    ),
  );
}
