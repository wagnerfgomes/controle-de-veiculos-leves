PRAGMA foreign_keys = ON;

CREATE TABLE motoristas (
  id        INTEGER PRIMARY KEY,
  nome      TEXT NOT NULL,
  nome_norm TEXT NOT NULL UNIQUE,
  matricula TEXT UNIQUE,
  setor     TEXT,
  ativo     INTEGER NOT NULL DEFAULT 1,
  criado_em TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE veiculos (
  id     INTEGER PRIMARY KEY,
  frota  TEXT NOT NULL UNIQUE,
  placa  TEXT UNIQUE,
  modelo TEXT,
  marca  TEXT,
  ano    INTEGER,
  tipo   TEXT,
  hodometro_atual INTEGER,
  ativo  INTEGER NOT NULL DEFAULT 1,
  observacao TEXT,
  criado_em TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE saidas (
  id           INTEGER PRIMARY KEY,
  veiculo_id   INTEGER NOT NULL REFERENCES veiculos(id),
  motorista_id INTEGER NOT NULL REFERENCES motoristas(id),
  turno        TEXT NOT NULL CHECK (turno IN ('A','B','C')),
  destino      TEXT,
  atividade    TEXT NOT NULL,
  dt_saida     TEXT NOT NULL,
  dt_chegada   TEXT,
  hodometro_saida   INTEGER,
  hodometro_chegada INTEGER,
  chegada_manual    INTEGER NOT NULL DEFAULT 0,
  observacao   TEXT,
  excluida_em  TEXT,
  excluida_por TEXT,
  excluida_motivo TEXT,
  criado_em    TEXT NOT NULL DEFAULT (datetime('now','localtime')),
  atualizado_em TEXT,
  CHECK (dt_chegada IS NULL OR dt_chegada > dt_saida),
  CHECK (hodometro_chegada IS NULL OR hodometro_saida IS NULL
         OR hodometro_chegada >= hodometro_saida)
);

-- As duas regras de ouro, garantidas pelo banco:
CREATE UNIQUE INDEX ux_veiculo_em_uso ON saidas(veiculo_id)
  WHERE dt_chegada IS NULL AND excluida_em IS NULL;
CREATE UNIQUE INDEX ux_motorista_em_uso ON saidas(motorista_id)
  WHERE dt_chegada IS NULL AND excluida_em IS NULL;

CREATE INDEX ix_saidas_periodo   ON saidas(dt_saida);
CREATE INDEX ix_saidas_veiculo   ON saidas(veiculo_id, dt_saida);
CREATE INDEX ix_saidas_motorista ON saidas(motorista_id, dt_saida);

CREATE TABLE auditoria (
  id INTEGER PRIMARY KEY,
  entidade TEXT NOT NULL,
  entidade_id INTEGER,
  acao TEXT NOT NULL,
  antes TEXT, depois TEXT,
  usuario_windows TEXT, maquina TEXT,
  em TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE config (chave TEXT PRIMARY KEY, valor TEXT);

INSERT INTO config (chave, valor) VALUES
  ('ultimo_backup_em', ''),
  ('ultimo_relatorio_periodo', ''),
  ('data_corte', date('now','localtime'));

PRAGMA user_version = 1;
