//! Contract / sandbox helpers for mensagem vs mensagens field name.

/// Decide which JSON key to lock after PDF inspection.
pub fn decidir_campo_mensagem(
    mensagem_apareceu_no_pdf: bool,
    mensagens_apareceu_no_pdf: bool,
) -> Result<&'static str, &'static str> {
    match (mensagem_apareceu_no_pdf, mensagens_apareceu_no_pdf) {
        (true, false) => Ok("mensagem"),
        (false, true) => Ok("mensagens"),
        (true, true) => Ok("mensagens"), // examples JSON use mensagens
        (false, false) => Err(
            "Nenhum marcador apareceu no PDF — escalar ao Sicredi antes da produção",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::boleto_status::parse_has_next;
    use serde_json::json;

    #[test]
    fn decide_mensagem_only() {
        assert_eq!(decidir_campo_mensagem(true, false).unwrap(), "mensagem");
    }

    #[test]
    fn decide_mensagens_only() {
        assert_eq!(decidir_campo_mensagem(false, true).unwrap(), "mensagens");
    }

    #[test]
    fn both_prefer_examples_json() {
        assert_eq!(decidir_campo_mensagem(true, true).unwrap(), "mensagens");
    }

    #[test]
    fn neither_blocks_production() {
        assert!(decidir_campo_mensagem(false, false).is_err());
    }

    #[test]
    fn has_next_contract() {
        assert_eq!(parse_has_next(&json!(false)), Some(false));
        assert_eq!(parse_has_next(&json!("false")), Some(false));
        assert_eq!(parse_has_next(&json!(true)), Some(true));
        assert_eq!(parse_has_next(&json!("true")), Some(true));
    }
}
