import { chamar } from "./cliente";
import type { Motorista, ParDuplicata } from "../tipos";

export const listarMotoristas = (incluirInativos = false) =>
  chamar<Motorista[]>("listar_motoristas", { incluirInativos });

export const buscarMotoristas = (termo: string) =>
  chamar<Motorista[]>("buscar_motoristas", { termo });

/** Com `confirmado: false` o Rust devolve `SEMELHANTE` sem gravar nada. A tela
 *  pergunta e só então reenvia confirmado. */
export const criarMotorista = (
  nome: string,
  matricula: string | null,
  setor: string | null,
  confirmado: boolean,
) => chamar<Motorista>("criar_motorista", { nome, matricula, setor, confirmado });

export const atualizarMotorista = (
  id: number,
  nome: string,
  matricula: string | null,
  setor: string | null,
  ativo: boolean,
) => chamar<Motorista>("atualizar_motorista", { id, nome, matricula, setor, ativo });

export const inativarMotorista = (id: number) =>
  chamar<void>("inativar_motorista", { id });

export const fundirMotoristas = (manterId: number, removerId: number) =>
  chamar<number>("fundir_motoristas", { manterId, removerId });

export const possiveisDuplicatasMotoristas = () =>
  chamar<ParDuplicata<Motorista>[]>("possiveis_duplicatas_motoristas");
