import { chamar } from "./cliente";
import type { FiltroSaidas, Formato, RelatorioDados, TipoRelatorio } from "../tipos";

export const previaRelatorio = (tipo: TipoRelatorio, filtro: FiltroSaidas) =>
  chamar<RelatorioDados>("previa_relatorio", { tipo, filtro });

/** Devolve o caminho do arquivo gravado em `relatorios\AAAA-MM\`. */
export const gerarRelatorio = (
  tipo: TipoRelatorio,
  filtro: FiltroSaidas,
  formato: Formato,
) => chamar<string>("gerar_relatorio", { tipo, filtro, formato });

export interface DetalheViagem {
  motorista: string;
  dt_saida: string;
  dt_chegada: string | null;
  turno: string;
  destino: string | null;
  atividade: string;
  chegada_manual: boolean;
  horas: number | null;
}

export const detalheVeiculoRelatorio = (veiculoId: number, filtro: FiltroSaidas) =>
  chamar<DetalheViagem[]>("detalhe_veiculo_relatorio", { veiculoId, filtro });
