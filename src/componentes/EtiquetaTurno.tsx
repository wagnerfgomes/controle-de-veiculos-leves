import type { Turno } from "../tipos";

export function EtiquetaTurno({ turno }: { turno: Turno }) {
  return <span className={`etiqueta-turno ${turno}`}>{turno}</span>;
}
