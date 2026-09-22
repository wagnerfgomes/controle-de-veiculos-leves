//! Tudo que toca o arquivo do banco.
//!
//! O banco é o arquivo que mora na pasta de rede. Não existe cópia local, fila
//! de escrita nem thread de sincronismo, e nada disso deve ser acrescentado: a
//! durabilidade vem de `synchronous = FULL` em [`conexao`].

pub mod auditoria;
pub mod backup;
pub mod conexao;
pub mod migracoes;
pub mod relatorios;
