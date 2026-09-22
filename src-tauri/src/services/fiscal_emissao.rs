//! Emissão fiscal não grava em `autobo_nfes`.
//! O registro autoritativo fica no AutoPlatform.

/// A emissão ainda não existe neste repositório. Quando existir, não aponta para a tabela de importação.
pub fn tabela_de_emissao() -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::tabela_de_emissao;

    #[test]
    fn emissao_nao_usa_autobo_nfes() {
        assert_ne!(tabela_de_emissao(), Some("autobo_nfes"));
    }
}
