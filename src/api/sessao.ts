import { chamar } from "./cliente";
import type { EstadoSessao, InfoBloqueio } from "../tipos";

export const iniciarSessao = () => chamar<EstadoSessao>("iniciar_sessao");

export const estadoSessao = () => chamar<EstadoSessao>("estado_sessao");

export const infoBloqueio = () => chamar<InfoBloqueio | null>("info_bloqueio");

export const tomarPosseLock = (confirmacao: string) =>
  chamar<EstadoSessao>("tomar_posse_lock", { confirmacao });

export const forcarBackup = () => chamar<string>("forcar_backup");

export const encerrarSessao = () => chamar<void>("encerrar_sessao");
