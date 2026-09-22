import { chamar } from "./cliente";
import type { ParDuplicata, Veiculo, VeiculoDisponibilidade } from "../tipos";

export interface DadosVeiculo {
  frota: string;
  placa: string | null;
  modelo: string | null;
  marca: string | null;
  ano: number | null;
  tipo: string | null;
  hodometroAtual: number | null;
  observacao: string | null;
}

export const listarVeiculos = (incluirInativos = false) =>
  chamar<Veiculo[]>("listar_veiculos", { incluirInativos });

/** Traz também os ocupados, com quem está e desde quando: o combo os mostra
 *  esmaecidos em vez de escondê-los. */
export const listarVeiculosDisponiveis = () =>
  chamar<VeiculoDisponibilidade[]>("listar_veiculos_disponiveis");

export const buscarVeiculos = (termo: string) =>
  chamar<Veiculo[]>("buscar_veiculos", { termo });

export const criarVeiculo = (dados: DadosVeiculo, confirmado: boolean) =>
  chamar<Veiculo>("criar_veiculo", { ...dados, confirmado });

export const atualizarVeiculo = (id: number, dados: DadosVeiculo, ativo: boolean) =>
  chamar<Veiculo>("atualizar_veiculo", { id, ...dados, ativo });

export const inativarVeiculo = (id: number) => chamar<void>("inativar_veiculo", { id });

export const fundirVeiculos = (manterId: number, removerId: number) =>
  chamar<number>("fundir_veiculos", { manterId, removerId });

export const possiveisDuplicatasVeiculos = () =>
  chamar<ParDuplicata<Veiculo>[]>("possiveis_duplicatas_veiculos");
