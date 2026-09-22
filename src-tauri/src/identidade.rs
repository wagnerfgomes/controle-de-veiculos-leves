//! Quem está operando e de onde.
//!
//! Não existe login: o sistema roda de uma pasta de rede e confia no usuário do
//! Windows. É esse nome que vai para a auditoria, para o `controle.lock.info` e
//! para o rodapé do relatório.

pub fn usuario() -> String {
    let nome = if cfg!(windows) {
        std::env::var("USERNAME").ok()
    } else {
        std::env::var("USER").ok()
    };
    let dominio = std::env::var("USERDOMAIN").ok();

    match (dominio, nome) {
        (Some(d), Some(n)) if !d.is_empty() && !n.is_empty() => format!("{d}\\{n}"),
        (_, Some(n)) if !n.is_empty() => n,
        _ => "desconhecido".to_string(),
    }
}

pub fn maquina() -> String {
    if let Ok(nome) = std::env::var("COMPUTERNAME") {
        if !nome.is_empty() {
            return nome;
        }
    }
    if let Ok(nome) = std::env::var("HOSTNAME") {
        if !nome.is_empty() {
            return nome;
        }
    }
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "desconhecida".to_string())
}
