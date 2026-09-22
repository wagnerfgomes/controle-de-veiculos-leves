//! Regra de negócio pura: nada aqui abre conexão, nada aqui sabe de Tauri.
//!
//! É o que permite testar o que importa sem banco. Regra nova entra aqui, nunca
//! em `comandos/`.

pub mod datahora;
pub mod duracao;
pub mod nome;
pub mod relatorio;
pub mod semelhanca;
pub mod validacao;
