import { useEffect, useMemo, useState } from "react";

import { comoErroApp, exigeConfirmacao } from "../api/cliente";
import { criarMotorista, listarMotoristas } from "../api/motoristas";
import { abrirSaida, destinosRecentes } from "../api/saidas";
import { criarVeiculo, listarVeiculosDisponiveis } from "../api/veiculos";
import { Aviso } from "../componentes/Aviso";
import { ComboBusca } from "../componentes/ComboBusca";
import { Modal } from "../componentes/Modal";
import type { ErroApp, Motorista, Turno, VeiculoDisponibilidade } from "../tipos";

interface Props {
  onFechar: () => void;
  onGravou: () => void;
}

/** Os campos estão na sequência em que a informação chega pelo rádio. */
export function NovaSaida({ onFechar, onGravou }: Props) {
  const [veiculos, setVeiculos] = useState<VeiculoDisponibilidade[]>([]);
  const [motoristas, setMotoristas] = useState<Motorista[]>([]);
  const [destinos, setDestinos] = useState<string[]>([]);

  const [veiculoId, setVeiculoId] = useState<number | null>(null);
  const [motoristaId, setMotoristaId] = useState<number | null>(null);
  const [turno, setTurno] = useState<Turno>(turnoPeloRelogio());
  const [dtSaida, setDtSaida] = useState(agoraLocal());
  const [destino, setDestino] = useState("");
  const [atividade, setAtividade] = useState("");
  const [hodometro, setHodometro] = useState("");
  const [observacao, setObservacao] = useState("");

  const [erro, setErro] = useState<ErroApp | null>(null);
  const [gravando, setGravando] = useState(false);
  const [cadastro, setCadastro] = useState<
    { tipo: "veiculo" | "motorista"; termo: string } | null
  >(null);

  const recarregar = () =>
    Promise.all([listarVeiculosDisponiveis(), listarMotoristas(), destinosRecentes()])
      .then(([v, m, d]) => {
        setVeiculos(v);
        setMotoristas(m);
        setDestinos(d);
      })
      .catch((e) => setErro(comoErroApp(e)));

  useEffect(() => {
    void recarregar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Pré-preenche o hodômetro com o último conhecido do veículo escolhido.
  const veiculoEscolhido = veiculos.find((v) => v.veiculo.id === veiculoId);
  useEffect(() => {
    if (veiculoEscolhido?.veiculo.hodometro_atual != null) {
      setHodometro(String(veiculoEscolhido.veiculo.hodometro_atual));
    }
  }, [veiculoEscolhido]);

  const itensVeiculo = useMemo(
    () =>
      veiculos.map((v) => ({
        id: v.veiculo.id,
        rotulo: v.veiculo.frota,
        detalhe: v.disponivel
          ? (v.veiculo.modelo ?? null)
          : `em viagem com ${v.motorista_atual} desde ${v.desde}`,
        indisponivel: !v.disponivel,
      })),
    [veiculos],
  );

  const itensMotorista = useMemo(
    () =>
      motoristas.map((m) => ({
        id: m.id,
        rotulo: m.nome,
        detalhe: m.matricula,
      })),
    [motoristas],
  );

  const gravar = async (confirmadoAvisos: boolean) => {
    if (veiculoId === null || motoristaId === null) {
      setErro({
        codigo: "VALIDACAO",
        mensagem: "Escolha o veículo e o condutor.",
      });
      return;
    }

    setGravando(true);
    try {
      await abrirSaida(
        {
          veiculoId,
          motoristaId,
          turno,
          destino: destino.trim() || null,
          atividade,
          dtSaida,
          hodometroSaida: hodometro.trim() ? Number(hodometro) : null,
          observacao: observacao.trim() || null,
        },
        confirmadoAvisos,
      );
      onGravou();
    } catch (bruto) {
      const e = comoErroApp(bruto);
      // Aviso de confirmação não é recusa: a tela pergunta e reenvia.
      if (exigeConfirmacao(e) && window.confirm(`${e.mensagem}\n\nConfirma?`)) {
        setGravando(false);
        await gravar(true);
        return;
      }
      setErro(e);
    } finally {
      setGravando(false);
    }
  };

  return (
    <Modal
      titulo="Nova saída"
      largura={620}
      onFechar={onFechar}
      rodape={
        <>
          <button type="button" onClick={onFechar}>
            Cancelar
          </button>
          <button
            type="button"
            className="verde grande"
            disabled={gravando}
            onClick={() => void gravar(false)}
          >
            {gravando ? "Registrando…" : "Registrar saída"}
          </button>
        </>
      }
    >
      {erro && <Aviso erro={erro} onFechar={() => setErro(null)} />}

      <ComboBusca
        rotulo="Veículo"
        itens={itensVeiculo}
        valor={veiculoId}
        onEscolher={setVeiculoId}
        onCadastrar={(termo) => setCadastro({ tipo: "veiculo", termo })}
        autoFocus
        placeholder="Frota ou modelo"
      />

      <ComboBusca
        rotulo="Condutor"
        itens={itensMotorista}
        valor={motoristaId}
        onEscolher={setMotoristaId}
        onCadastrar={(termo) => setCadastro({ tipo: "motorista", termo })}
        placeholder="Nome ou matrícula"
      />

      <div className="linha">
        <div className="campo">
          <label>Turno</label>
          <div className="botoes-turno">
            {(["A", "B", "C"] as const).map((t) => (
              <button
                key={t}
                type="button"
                className={turno === t ? `turno-escolhido ${t}` : ""}
                onClick={() => setTurno(t)}
              >
                {t}
              </button>
            ))}
          </div>
        </div>

        <div className="campo">
          <label>Saída</label>
          <input
            type="datetime-local"
            value={dtSaida.replace(" ", "T")}
            onChange={(e) => setDtSaida(e.target.value.replace("T", " ").slice(0, 16))}
          />
        </div>
      </div>

      <div className="campo">
        <label>Destino</label>
        <input
          list="destinos-anteriores"
          value={destino}
          onChange={(e) => setDestino(e.target.value)}
          placeholder="Opcional"
        />
        <datalist id="destinos-anteriores">
          {destinos.map((d) => (
            <option key={d} value={d} />
          ))}
        </datalist>
      </div>

      <div className="campo">
        <label>Atividade</label>
        <input
          value={atividade}
          onChange={(e) => setAtividade(e.target.value)}
          placeholder="O que vai fazer"
        />
      </div>

      <div className="linha">
        <div className="campo">
          <label>Hodômetro</label>
          <input
            type="number"
            value={hodometro}
            onChange={(e) => setHodometro(e.target.value)}
            placeholder="Opcional"
          />
        </div>
        <div className="campo">
          <label>Observação</label>
          <input
            value={observacao}
            onChange={(e) => setObservacao(e.target.value)}
            placeholder="Opcional"
          />
        </div>
      </div>

      {cadastro && (
        <CadastroRapido
          tipo={cadastro.tipo}
          termo={cadastro.termo}
          onFechar={() => setCadastro(null)}
          onCriou={async (id) => {
            await recarregar();
            if (cadastro.tipo === "veiculo") setVeiculoId(id);
            else setMotoristaId(id);
            setCadastro(null);
          }}
        />
      )}
    </Modal>
  );
}

/** Mini-formulário sem sair do modal. Com `confirmado: false` o Rust devolve
 *  `SEMELHANTE` sem gravar nada, e só então perguntamos. */
function CadastroRapido({
  tipo,
  termo,
  onFechar,
  onCriou,
}: {
  tipo: "veiculo" | "motorista";
  termo: string;
  onFechar: () => void;
  onCriou: (id: number) => void | Promise<void>;
}) {
  const [valor, setValor] = useState(termo);
  const [complemento, setComplemento] = useState("");
  const [erro, setErro] = useState<ErroApp | null>(null);

  const criar = async (confirmado: boolean) => {
    try {
      if (tipo === "veiculo") {
        const v = await criarVeiculo(
          {
            frota: valor,
            placa: null,
            modelo: complemento.trim() || null,
            marca: null,
            ano: null,
            tipo: null,
            hodometroAtual: null,
            observacao: null,
          },
          confirmado,
        );
        await onCriou(v.id);
        return;
      }
      const m = await criarMotorista(valor, complemento.trim() || null, null, confirmado);
      await onCriou(m.id);
    } catch (bruto) {
      const e = comoErroApp(bruto);
      if (e.codigo === "SEMELHANTE" && window.confirm(`${e.mensagem}\n\nCadastrar assim mesmo?`)) {
        await criar(true);
        return;
      }
      setErro(e);
    }
  };

  return (
    <Modal
      titulo={tipo === "veiculo" ? "Cadastrar veículo" : "Cadastrar condutor"}
      largura={420}
      onFechar={onFechar}
      rodape={
        <>
          <button type="button" onClick={onFechar}>
            Cancelar
          </button>
          <button type="button" className="primario" onClick={() => void criar(false)}>
            Cadastrar
          </button>
        </>
      }
    >
      {erro && <Aviso erro={erro} onFechar={() => setErro(null)} />}
      <div className="campo">
        <label>{tipo === "veiculo" ? "Frota" : "Nome"}</label>
        <input value={valor} onChange={(e) => setValor(e.target.value)} autoFocus />
      </div>
      <div className="campo">
        <label>{tipo === "veiculo" ? "Modelo" : "Matrícula"}</label>
        <input
          value={complemento}
          onChange={(e) => setComplemento(e.target.value)}
          placeholder="Opcional"
        />
      </div>
    </Modal>
  );
}

/** Pré-seleção pelo horário, e só isso: a regra real depende da escala, não do
 *  relógio. A base antiga mostra turno A às 03:00 e C às 12:00, então derivar
 *  seria errado. O campo continua editável. */
function turnoPeloRelogio(): Turno {
  const hora = new Date().getHours();
  if (hora < 7) return "C";
  if (hora < 15) return "A";
  return "B";
}

function agoraLocal(): string {
  const agora = new Date();
  const doisDigitos = (n: number) => String(n).padStart(2, "0");
  return (
    `${agora.getFullYear()}-${doisDigitos(agora.getMonth() + 1)}-${doisDigitos(agora.getDate())}` +
    ` ${doisDigitos(agora.getHours())}:${doisDigitos(agora.getMinutes())}`
  );
}
