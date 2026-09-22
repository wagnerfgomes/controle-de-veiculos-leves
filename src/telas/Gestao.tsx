import { useCallback, useEffect, useState } from "react";

import { comoErroApp } from "../api/cliente";
import {
  atualizarMotorista,
  criarMotorista,
  fundirMotoristas,
  inativarMotorista,
  listarMotoristas,
  possiveisDuplicatasMotoristas,
} from "../api/motoristas";
import {
  atualizarVeiculo,
  criarVeiculo,
  fundirVeiculos,
  inativarVeiculo,
  listarVeiculos,
  possiveisDuplicatasVeiculos,
} from "../api/veiculos";
import { Aviso, Toast } from "../componentes/Aviso";
import { Modal } from "../componentes/Modal";
import type { ErroApp, Motorista, ParDuplicata, Veiculo } from "../tipos";

type Aba = "motoristas" | "veiculos";

/** Com base nova e ninguém cadastrado, esta é a porta de entrada do sistema.
 *  O indicador de duplicatas no topo é a rede de segurança do cadastro rápido:
 *  transforma a limpeza em hábito em vez de mutirão. */
export function Gestao() {
  const [aba, setAba] = useState<Aba>("veiculos");
  const [erro, setErro] = useState<ErroApp | null>(null);
  const [aviso, setAviso] = useState<string | null>(null);

  return (
    <div className="gestao">
      {erro && <Aviso erro={erro} onFechar={() => setErro(null)} />}
      {aviso && <Toast texto={aviso} onFechar={() => setAviso(null)} />}

      <nav className="abas">
        <button
          type="button"
          className={aba === "veiculos" ? "ativa" : ""}
          onClick={() => setAba("veiculos")}
        >
          Veículos
        </button>
        <button
          type="button"
          className={aba === "motoristas" ? "ativa" : ""}
          onClick={() => setAba("motoristas")}
        >
          Condutores
        </button>
      </nav>

      {aba === "veiculos" ? (
        <AbaVeiculos aoErro={setErro} aoAvisar={setAviso} />
      ) : (
        <AbaMotoristas aoErro={setErro} aoAvisar={setAviso} />
      )}
    </div>
  );
}

interface PropsAba {
  aoErro: (e: ErroApp) => void;
  aoAvisar: (texto: string) => void;
}

function AbaVeiculos({ aoErro, aoAvisar }: PropsAba) {
  const [itens, setItens] = useState<Veiculo[]>([]);
  const [duplicatas, setDuplicatas] = useState<ParDuplicata<Veiculo>[]>([]);
  const [editando, setEditando] = useState<Veiculo | "novo" | null>(null);

  const recarregar = useCallback(() => {
    Promise.all([listarVeiculos(true), possiveisDuplicatasVeiculos()])
      .then(([v, d]) => {
        setItens(v);
        setDuplicatas(d);
      })
      .catch((e) => aoErro(comoErroApp(e)));
  }, [aoErro]);

  useEffect(recarregar, [recarregar]);

  const fundir = async (par: ParDuplicata<Veiculo>) => {
    const manter = window.confirm(
      `Fundir ${par.b.frota} em ${par.a.frota}?\n\n` +
        `As viagens de ${par.b.frota} passam para ${par.a.frota}, e ${par.b.frota} é removido.`,
    );
    if (!manter) return;

    try {
      const reapontadas = await fundirVeiculos(par.a.id, par.b.id);
      aoAvisar(`${reapontadas} viagem(ns) reapontada(s) para ${par.a.frota}.`);
      recarregar();
    } catch (bruto) {
      aoErro(comoErroApp(bruto));
    }
  };

  return (
    <>
      <Duplicatas
        pares={duplicatas}
        rotulo={(v) => v.frota}
        onFundir={(par) => void fundir(par)}
      />

      <section className="cartao bloco">
        <div className="cabecalho-bloco">
          <h2>
            Veículos <span className="sutil">({itens.length})</span>
          </h2>
          <button type="button" className="primario" onClick={() => setEditando("novo")}>
            + Cadastrar veículo
          </button>
        </div>

        <table>
          <thead>
            <tr>
              <th>Frota</th>
              <th>Modelo</th>
              <th>Placa</th>
              <th>Tipo</th>
              <th className="num">Hodômetro</th>
              <th>Situação</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {itens.map((v) => (
              <tr key={v.id} className={v.ativo ? "" : "inativo"}>
                <td>
                  <strong>{v.frota}</strong>
                </td>
                <td>{v.modelo ?? "—"}</td>
                <td>{v.placa ?? "—"}</td>
                <td>{v.tipo ?? "—"}</td>
                <td className="num">{v.hodometro_atual ?? "—"}</td>
                <td>{v.ativo ? "Ativo" : "Inativo"}</td>
                <td className="num">
                  <button type="button" onClick={() => setEditando(v)}>
                    Editar
                  </button>
                  {v.ativo && (
                    <button
                      type="button"
                      onClick={() =>
                        inativarVeiculo(v.id)
                          .then(recarregar)
                          .catch((e) => aoErro(comoErroApp(e)))
                      }
                    >
                      Inativar
                    </button>
                  )}
                </td>
              </tr>
            ))}
            {itens.length === 0 && (
              <tr>
                <td colSpan={7} className="sutil vazio">
                  Nenhum veículo cadastrado ainda.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>

      {editando && (
        <FormularioVeiculo
          veiculo={editando === "novo" ? null : editando}
          onFechar={() => setEditando(null)}
          onGravou={() => {
            setEditando(null);
            recarregar();
          }}
          aoErro={aoErro}
        />
      )}
    </>
  );
}

function AbaMotoristas({ aoErro, aoAvisar }: PropsAba) {
  const [itens, setItens] = useState<Motorista[]>([]);
  const [duplicatas, setDuplicatas] = useState<ParDuplicata<Motorista>[]>([]);
  const [editando, setEditando] = useState<Motorista | "novo" | null>(null);

  const recarregar = useCallback(() => {
    Promise.all([listarMotoristas(true), possiveisDuplicatasMotoristas()])
      .then(([m, d]) => {
        setItens(m);
        setDuplicatas(d);
      })
      .catch((e) => aoErro(comoErroApp(e)));
  }, [aoErro]);

  useEffect(recarregar, [recarregar]);

  const fundir = async (par: ParDuplicata<Motorista>) => {
    const ok = window.confirm(
      `Fundir ${par.b.nome} em ${par.a.nome}?\n\n` +
        `As viagens de ${par.b.nome} passam para ${par.a.nome}, e o cadastro duplicado é removido.`,
    );
    if (!ok) return;

    try {
      const reapontadas = await fundirMotoristas(par.a.id, par.b.id);
      aoAvisar(`${reapontadas} viagem(ns) reapontada(s) para ${par.a.nome}.`);
      recarregar();
    } catch (bruto) {
      aoErro(comoErroApp(bruto));
    }
  };

  return (
    <>
      <Duplicatas pares={duplicatas} rotulo={(m) => m.nome} onFundir={(p) => void fundir(p)} />

      <section className="cartao bloco">
        <div className="cabecalho-bloco">
          <h2>
            Condutores <span className="sutil">({itens.length})</span>
          </h2>
          <button type="button" className="primario" onClick={() => setEditando("novo")}>
            + Cadastrar condutor
          </button>
        </div>

        <table>
          <thead>
            <tr>
              <th>Nome</th>
              <th>Matrícula</th>
              <th>Setor</th>
              <th>Situação</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {itens.map((m) => (
              <tr key={m.id} className={m.ativo ? "" : "inativo"}>
                <td>
                  <strong>{m.nome}</strong>
                </td>
                <td>{m.matricula ?? "—"}</td>
                <td>{m.setor ?? "—"}</td>
                <td>{m.ativo ? "Ativo" : "Inativo"}</td>
                <td className="num">
                  <button type="button" onClick={() => setEditando(m)}>
                    Editar
                  </button>
                  {m.ativo && (
                    <button
                      type="button"
                      onClick={() =>
                        inativarMotorista(m.id)
                          .then(recarregar)
                          .catch((e) => aoErro(comoErroApp(e)))
                      }
                    >
                      Inativar
                    </button>
                  )}
                </td>
              </tr>
            ))}
            {itens.length === 0 && (
              <tr>
                <td colSpan={5} className="sutil vazio">
                  Nenhum condutor cadastrado ainda.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>

      {editando && (
        <FormularioMotorista
          motorista={editando === "novo" ? null : editando}
          onFechar={() => setEditando(null)}
          onGravou={() => {
            setEditando(null);
            recarregar();
          }}
          aoErro={aoErro}
        />
      )}
    </>
  );
}

function Duplicatas<T>({
  pares,
  rotulo,
  onFundir,
}: {
  pares: ParDuplicata<T>[];
  rotulo: (item: T) => string;
  onFundir: (par: ParDuplicata<T>) => void;
}) {
  if (pares.length === 0) return null;

  return (
    <section className="cartao bloco duplicatas">
      <h2>
        Possíveis duplicatas <span className="sutil">({pares.length})</span>
      </h2>
      <ul>
        {pares.map((par, i) => (
          <li key={i}>
            <span>
              <strong>{rotulo(par.a)}</strong> e <strong>{rotulo(par.b)}</strong>{" "}
              <span className="sutil">(distância {par.distancia})</span>
            </span>
            <button type="button" onClick={() => onFundir(par)}>
              Fundir
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

function FormularioVeiculo({
  veiculo,
  onFechar,
  onGravou,
  aoErro,
}: {
  veiculo: Veiculo | null;
  onFechar: () => void;
  onGravou: () => void;
  aoErro: (e: ErroApp) => void;
}) {
  const [frota, setFrota] = useState(veiculo?.frota ?? "");
  const [modelo, setModelo] = useState(veiculo?.modelo ?? "");
  const [marca, setMarca] = useState(veiculo?.marca ?? "");
  const [placa, setPlaca] = useState(veiculo?.placa ?? "");
  const [ano, setAno] = useState(veiculo?.ano ? String(veiculo.ano) : "");
  const [tipo, setTipo] = useState(veiculo?.tipo ?? "");
  const [hodometro, setHodometro] = useState(
    veiculo?.hodometro_atual ? String(veiculo.hodometro_atual) : "",
  );
  const [observacao, setObservacao] = useState(veiculo?.observacao ?? "");
  const [ativo, setAtivo] = useState(veiculo?.ativo ?? true);

  const gravar = async (confirmado: boolean) => {
    const dados = {
      frota,
      placa: placa.trim() || null,
      modelo: modelo.trim() || null,
      marca: marca.trim() || null,
      ano: ano.trim() ? Number(ano) : null,
      tipo: tipo.trim() || null,
      hodometroAtual: hodometro.trim() ? Number(hodometro) : null,
      observacao: observacao.trim() || null,
    };

    try {
      if (veiculo) await atualizarVeiculo(veiculo.id, dados, ativo);
      else await criarVeiculo(dados, confirmado);
      onGravou();
    } catch (bruto) {
      const e = comoErroApp(bruto);
      if (e.codigo === "SEMELHANTE" && window.confirm(`${e.mensagem}\n\nCadastrar assim mesmo?`)) {
        await gravar(true);
        return;
      }
      aoErro(e);
    }
  };

  return (
    <Modal
      titulo={veiculo ? `Editar ${veiculo.frota}` : "Cadastrar veículo"}
      onFechar={onFechar}
      rodape={
        <>
          <button type="button" onClick={onFechar}>
            Cancelar
          </button>
          <button type="button" className="primario" onClick={() => void gravar(false)}>
            Gravar
          </button>
        </>
      }
    >
      <div className="linha">
        <Campo rotulo="Frota" valor={frota} aoMudar={setFrota} autoFocus />
        <Campo rotulo="Placa" valor={placa} aoMudar={setPlaca} />
      </div>
      <div className="linha">
        <Campo rotulo="Modelo" valor={modelo} aoMudar={setModelo} />
        <Campo rotulo="Marca" valor={marca} aoMudar={setMarca} />
      </div>
      <div className="linha">
        <Campo rotulo="Ano" valor={ano} aoMudar={setAno} tipo="number" />
        <Campo rotulo="Tipo" valor={tipo} aoMudar={setTipo} />
        <Campo rotulo="Hodômetro" valor={hodometro} aoMudar={setHodometro} tipo="number" />
      </div>
      <Campo rotulo="Observação" valor={observacao} aoMudar={setObservacao} />
      {veiculo && (
        <label className="caixa">
          <input type="checkbox" checked={ativo} onChange={(e) => setAtivo(e.target.checked)} />{" "}
          Ativo
        </label>
      )}
    </Modal>
  );
}

function FormularioMotorista({
  motorista,
  onFechar,
  onGravou,
  aoErro,
}: {
  motorista: Motorista | null;
  onFechar: () => void;
  onGravou: () => void;
  aoErro: (e: ErroApp) => void;
}) {
  const [nome, setNome] = useState(motorista?.nome ?? "");
  const [matricula, setMatricula] = useState(motorista?.matricula ?? "");
  const [setor, setSetor] = useState(motorista?.setor ?? "");
  const [ativo, setAtivo] = useState(motorista?.ativo ?? true);

  const gravar = async (confirmado: boolean) => {
    try {
      if (motorista) {
        await atualizarMotorista(
          motorista.id,
          nome,
          matricula.trim() || null,
          setor.trim() || null,
          ativo,
        );
      } else {
        await criarMotorista(nome, matricula.trim() || null, setor.trim() || null, confirmado);
      }
      onGravou();
    } catch (bruto) {
      const e = comoErroApp(bruto);
      if (e.codigo === "SEMELHANTE" && window.confirm(`${e.mensagem}\n\nCadastrar assim mesmo?`)) {
        await gravar(true);
        return;
      }
      aoErro(e);
    }
  };

  return (
    <Modal
      titulo={motorista ? `Editar ${motorista.nome}` : "Cadastrar condutor"}
      largura={460}
      onFechar={onFechar}
      rodape={
        <>
          <button type="button" onClick={onFechar}>
            Cancelar
          </button>
          <button type="button" className="primario" onClick={() => void gravar(false)}>
            Gravar
          </button>
        </>
      }
    >
      <Campo rotulo="Nome" valor={nome} aoMudar={setNome} autoFocus />
      <div className="linha">
        <Campo rotulo="Matrícula" valor={matricula} aoMudar={setMatricula} />
        <Campo rotulo="Setor" valor={setor} aoMudar={setSetor} />
      </div>
      {motorista && (
        <label className="caixa">
          <input type="checkbox" checked={ativo} onChange={(e) => setAtivo(e.target.checked)} />{" "}
          Ativo
        </label>
      )}
    </Modal>
  );
}

function Campo({
  rotulo,
  valor,
  aoMudar,
  tipo,
  autoFocus,
}: {
  rotulo: string;
  valor: string;
  aoMudar: (v: string) => void;
  tipo?: string;
  autoFocus?: boolean;
}) {
  return (
    <div className="campo">
      <label>{rotulo}</label>
      <input
        type={tipo ?? "text"}
        value={valor}
        autoFocus={autoFocus}
        onChange={(e) => aoMudar(e.target.value)}
      />
    </div>
  );
}
