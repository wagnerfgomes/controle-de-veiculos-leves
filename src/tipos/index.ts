// Espelho das structs serde do Rust. Os nomes são iguais dos dois lados de
// propósito: tradução de nome entre camadas é erro que nenhum compilador pega.

export type Turno = "A" | "B" | "C";
export type Modo = "producao" | "demonstracao";
export type Conexao = "Normal" | "Degradada";

export interface ErroApp {
  codigo:
    | "VEICULO_EM_USO"
    | "MOTORISTA_EM_USO"
    | "DUPLICADO"
    | "SEMELHANTE"
    | "NAO_ENCONTRADO"
    | "VALIDACAO"
    | "BANCO"
    | "SESSAO_PERDIDA"
    | "CONEXAO_PERDIDA";
  mensagem: string;
  detalhe?: string | null;
}

export interface Motorista {
  id: number;
  nome: string;
  nome_norm: string;
  matricula: string | null;
  setor: string | null;
  ativo: boolean;
  criado_em: string;
}

export interface Veiculo {
  id: number;
  frota: string;
  placa: string | null;
  modelo: string | null;
  marca: string | null;
  ano: number | null;
  tipo: string | null;
  hodometro_atual: number | null;
  ativo: boolean;
  observacao: string | null;
  criado_em: string;
}

export interface Saida {
  id: number;
  veiculo_id: number;
  motorista_id: number;
  turno: Turno;
  destino: string | null;
  atividade: string;
  dt_saida: string;
  dt_chegada: string | null;
  hodometro_saida: number | null;
  hodometro_chegada: number | null;
  chegada_manual: boolean;
  observacao: string | null;
  excluida_em: string | null;
  excluida_por: string | null;
  excluida_motivo: string | null;
  criado_em: string;
  atualizado_em: string | null;
  frota: string | null;
  motorista: string | null;
}

export interface SaidaAberta {
  id: number;
  frota: string;
  motorista: string;
  destino: string | null;
  atividade: string;
  dt_saida: string;
  turno: Turno;
  horas_decorridas: number;
  /** Acima de 24 h. */
  alerta: boolean;
}

export interface VeiculoDisponibilidade {
  veiculo: Veiculo;
  disponivel: boolean;
  motorista_atual: string | null;
  desde: string | null;
  destino_atual: string | null;
}

export interface FiltroSaidas {
  de?: string | null;
  ate?: string | null;
  veiculo_id?: number | null;
  motorista_id?: number | null;
  turnos: Turno[];
  somente_abertas: boolean;
  incluir_excluidas: boolean;
}

export interface Kpis {
  viagens: number;
  abertas: number;
  condutores_distintos: number;
  frotas_distintas: number;
  /** `null` quando nenhuma viagem fechou: ausência de dado, não zero. */
  duracao_media_horas: number | null;
  por_turno: [number, number, number];
}

export interface ParDuplicata<T> {
  a: T;
  b: T;
  distancia: number;
}

export interface EstadoSessao {
  usuario_windows: string;
  maquina: string;
  inicio: string;
  conexao: Conexao;
  ultimo_backup: string | null;
  user_version: number;
  caminho_dados: string;
  modo: Modo;
  /** Falso fora do Windows: o lock é um stub e a barra de status precisa dizer. */
  lock_confiavel: boolean;
}

export interface InfoBloqueio {
  usuario_windows: string;
  maquina: string;
  desde: string;
  minutos_sem_heartbeat: number | null;
  pode_tomar_posse: boolean;
}

export type TipoRelatorio = "UsoVeiculo" | "UsoMotorista";
export type Formato = "Html" | "Pdf" | "Csv";

export interface CabecalhoRelatorio {
  de: string;
  ate: string;
  viagens: number;
  sem_chegada: number;
  sem_chegada_pct: number;
  chegadas_manuais: number;
  chegadas_manuais_pct: number;
  hodometro_completo: number;
  hodometro_completo_pct: number;
  emitido_em: string;
  emitido_por: string;
}

export interface LinhaRelatorio {
  rotulo: string;
  viagens: number;
  abertas: number;
  horas_totais: number;
  horas_media: number | null;
  chegadas_manuais: number;
  km: number | null;
  veiculos_distintos: number | null;
  frotas: string | null;
}

export interface RelatorioDados {
  tipo: TipoRelatorio;
  cabecalho: CabecalhoRelatorio;
  linhas: LinhaRelatorio[];
}

export const FILTRO_VAZIO: FiltroSaidas = {
  de: null,
  ate: null,
  veiculo_id: null,
  motorista_id: null,
  turnos: [],
  somente_abertas: false,
  incluir_excluidas: false,
};
