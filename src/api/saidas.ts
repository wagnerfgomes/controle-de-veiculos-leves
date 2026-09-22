import { chamar } from "./cliente";
import type { FiltroSaidas, Kpis, Saida, SaidaAberta, Turno } from "../tipos";

export interface DadosAbertura {
  veiculoId: number;
  motoristaId: number;
  turno: Turno;
  destino: string | null;
  atividade: string;
  /** `null` = agora. */
  dtSaida: string | null;
  hodometroSaida: number | null;
  observacao: string | null;
}

export const abrirSaida = (dados: DadosAbertura, confirmadoAvisos: boolean) =>
  chamar<Saida>("abrir_saida", { ...dados, confirmadoAvisos });

/** `dtChegada: null` = agora, e grava `chegada_manual = 0`. O caminho de um
 *  clique é o que evita a chegada nunca registrada. */
export const encerrarSaida = (
  id: number,
  dtChegada: string | null,
  hodometroChegada: number | null,
  confirmadoAvisos: boolean,
) => chamar<Saida>("encerrar_saida", { id, dtChegada, hodometroChegada, confirmadoAvisos });

export interface DadosEdicao {
  id: number;
  turno: Turno;
  destino: string | null;
  atividade: string;
  dtSaida: string;
  dtChegada: string | null;
  hodometroSaida: number | null;
  hodometroChegada: number | null;
  observacao: string | null;
}

export const editarSaida = (dados: DadosEdicao, confirmadoAvisos: boolean) =>
  chamar<Saida>("editar_saida", { ...dados, confirmadoAvisos });

/** Exclusão é lógica e o motivo é obrigatório: exclusão sem motivo é a que
 *  ninguém consegue explicar três meses depois. */
export const excluirSaida = (id: number, motivo: string) =>
  chamar<void>("excluir_saida", { id, motivo });

export const listarSaidasAbertas = () => chamar<SaidaAberta[]>("listar_saidas_abertas");

export const listarSaidas = (filtro: FiltroSaidas) =>
  chamar<Saida[]>("listar_saidas", { filtro });

export const kpis = (filtro: FiltroSaidas) => chamar<Kpis>("kpis", { filtro });

export const destinosRecentes = () => chamar<string[]>("destinos_recentes");
