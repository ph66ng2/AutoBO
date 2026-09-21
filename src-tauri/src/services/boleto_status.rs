//! Financial status machine for Sicredi-backed boletos.
//!
//! All transitions use compare-and-set (`WHERE status = expected`).
//! Precedence: PAGO (absolute) > CANCELADO > BAIXA_SOLICITADA > VENCIDO > REGISTRADO.
//! CANCELADO → PAGO only with authoritative bank liquidation evidence.

use serde::{Deserialize, Serialize};

/// Financial statuses persisted on `autobo_boletos.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatusFinanceiro {
    Rascunho,
    Registrando,
    Registrado,
    RegistroIndeterminado,
    ErroPayload,
    AguardandoRetry,
    Vencido,
    BaixaEnviando,
    BaixaSolicitada,
    BaixaIndeterminada,
    Pago,
    Cancelado,
}

impl StatusFinanceiro {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rascunho => "RASCUNHO",
            Self::Registrando => "REGISTRANDO",
            Self::Registrado => "REGISTRADO",
            Self::RegistroIndeterminado => "REGISTRO_INDETERMINADO",
            Self::ErroPayload => "ERRO_PAYLOAD",
            Self::AguardandoRetry => "AGUARDANDO_RETRY",
            Self::Vencido => "VENCIDO",
            Self::BaixaEnviando => "BAIXA_ENVIANDO",
            Self::BaixaSolicitada => "BAIXA_SOLICITADA",
            Self::BaixaIndeterminada => "BAIXA_INDETERMINADA",
            Self::Pago => "PAGO",
            Self::Cancelado => "CANCELADO",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "RASCUNHO" => Some(Self::Rascunho),
            "REGISTRANDO" => Some(Self::Registrando),
            "REGISTRADO" => Some(Self::Registrado),
            "REGISTRO_INDETERMINADO" => Some(Self::RegistroIndeterminado),
            "ERRO_PAYLOAD" => Some(Self::ErroPayload),
            "AGUARDANDO_RETRY" => Some(Self::AguardandoRetry),
            "VENCIDO" => Some(Self::Vencido),
            "BAIXA_ENVIANDO" => Some(Self::BaixaEnviando),
            "BAIXA_SOLICITADA" => Some(Self::BaixaSolicitada),
            "BAIXA_INDETERMINADA" => Some(Self::BaixaIndeterminada),
            "PAGO" => Some(Self::Pago),
            "CANCELADO" => Some(Self::Cancelado),
            _ => None,
        }
    }

    /// Rank for precedence. Higher wins. PAGO is absolute.
    pub fn rank(self) -> u8 {
        match self {
            Self::Pago => 100,
            Self::Cancelado => 90,
            Self::BaixaSolicitada => 50,
            Self::Vencido => 40,
            Self::Registrado => 30,
            // Intermediate / draft ranks — not used for "don't regress" among finals
            Self::BaixaEnviando | Self::BaixaIndeterminada => 25,
            Self::AguardandoRetry => 20,
            Self::Registrando | Self::RegistroIndeterminado | Self::ErroPayload => 10,
            Self::Rascunho => 0,
        }
    }

    pub fn is_absolutely_terminal(self) -> bool {
        matches!(self, Self::Pago)
    }

    /// Terminal for user commands (new baixa PATCH, restore to REGISTRADO).
    pub fn is_user_command_terminal(self) -> bool {
        matches!(self, Self::Pago | Self::Cancelado)
    }

    /// Origins allowed when applying authoritative bank liquidation → PAGO.
    pub fn can_become_pago_from_liquidation(self) -> bool {
        matches!(
            self,
            Self::Registrado
                | Self::Vencido
                | Self::BaixaEnviando
                | Self::BaixaSolicitada
                | Self::BaixaIndeterminada
                | Self::AguardandoRetry
                | Self::Cancelado // authoritative exception
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperacaoPendente {
    Cadastro,
    Baixa,
}

impl OperacaoPendente {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cadastro => "CADASTRO",
            Self::Baixa => "BAIXA",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "CADASTRO" => Some(Self::Cadastro),
            "BAIXA" => Some(Self::Baixa),
            _ => None,
        }
    }
}

/// Result of attempting a conditional transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransicaoResultado {
    Aplicada { de: StatusFinanceiro, para: StatusFinanceiro },
    RejeitadaPorCas { atual: StatusFinanceiro },
    BloqueadaPorPrecedencia { atual: StatusFinanceiro, tentou: StatusFinanceiro },
}

/// Decide whether `para` may replace `atual` when CAS missed / after re-read.
///
/// Special case: CANCELADO → PAGO only when `liquidacao_bancaria` is true.
pub fn pode_aplicar(
    atual: StatusFinanceiro,
    para: StatusFinanceiro,
    liquidacao_bancaria: bool,
) -> bool {
    if atual == para {
        return true;
    }
    if atual.is_absolutely_terminal() {
        return false;
    }
    if para == StatusFinanceiro::Pago {
        return liquidacao_bancaria && atual.can_become_pago_from_liquidation();
    }
    // Never regress from CANCELADO via restore / user ops
    if atual == StatusFinanceiro::Cancelado {
        return false;
    }
    // Don't overwrite a newer financial state with an older one
    if para.rank() < atual.rank() && !is_intermediate_progress(atual, para) {
        return false;
    }
    true
}

fn is_intermediate_progress(atual: StatusFinanceiro, para: StatusFinanceiro) -> bool {
    matches!(
        (atual, para),
        (
            StatusFinanceiro::BaixaEnviando,
            StatusFinanceiro::BaixaSolicitada
                | StatusFinanceiro::BaixaIndeterminada
                | StatusFinanceiro::AguardandoRetry
                | StatusFinanceiro::Cancelado
                | StatusFinanceiro::Pago
        ) | (
            StatusFinanceiro::BaixaIndeterminada,
            StatusFinanceiro::BaixaSolicitada
                | StatusFinanceiro::Cancelado
                | StatusFinanceiro::Pago
        ) | (
            StatusFinanceiro::Registrando,
            StatusFinanceiro::Registrado
                | StatusFinanceiro::RegistroIndeterminado
                | StatusFinanceiro::ErroPayload
                | StatusFinanceiro::AguardandoRetry
        ) | (
            StatusFinanceiro::AguardandoRetry,
            StatusFinanceiro::Registrando | StatusFinanceiro::BaixaEnviando
        ) | (
            StatusFinanceiro::RegistroIndeterminado,
            StatusFinanceiro::Registrado
        ) | (
            StatusFinanceiro::Rascunho | StatusFinanceiro::ErroPayload,
            StatusFinanceiro::Registrando
        ) | (
            StatusFinanceiro::Registrado | StatusFinanceiro::Vencido,
            StatusFinanceiro::BaixaEnviando | StatusFinanceiro::Vencido | StatusFinanceiro::Pago
        )
    )
}

/// After CAS affected 0 rows, decide next action from re-read status.
pub fn apos_cas_zero(
    atual: StatusFinanceiro,
    tentou: StatusFinanceiro,
    liquidacao_bancaria: bool,
) -> TransicaoResultado {
    if pode_aplicar(atual, tentou, liquidacao_bancaria) && atual != tentou {
        // Caller should retry CAS with expected = atual
        TransicaoResultado::RejeitadaPorCas { atual }
    } else if atual == tentou {
        TransicaoResultado::Aplicada {
            de: atual,
            para: tentou,
        }
    } else {
        TransicaoResultado::BloqueadaPorPrecedencia { atual, tentou }
    }
}

/// Whether a single EM CARTEIRA consultation may release a new baixa PATCH.
/// Plan: never — stay BAIXA_INDETERMINADA.
pub fn em_carteira_libera_novo_patch(_consultas_consecutivas: u32, _confirmacao_humana: bool) -> bool {
    // Explicit policy: even with repeats, human confirmation is required.
    // This helper returns false unless both conditions are met by the caller API.
    false
}

pub fn em_carteira_pode_liberar_patch(
    consultas_em_carteira_consecutivas: u32,
    confirmacao_humana: bool,
) -> bool {
    consultas_em_carteira_consecutivas >= 3 && confirmacao_humana
}

/// Map Sicredi situacao / liquidation evidence to target status.
pub fn status_de_situacao_sicredi(situacao: &str) -> Option<(StatusFinanceiro, bool)> {
    let s = situacao.trim().to_uppercase();
    let s = s.replace('_', " ");
    if s.starts_with("LIQUIDADO") {
        return Some((StatusFinanceiro::Pago, true));
    }
    if s.contains("BAIXADO") {
        return Some((StatusFinanceiro::Cancelado, false));
    }
    if s.contains("VENCIDO") {
        return Some((StatusFinanceiro::Vencido, false));
    }
    if s.contains("EM CARTEIRA") {
        return Some((StatusFinanceiro::Registrado, false));
    }
    if s.contains("REJEITADO") {
        return Some((StatusFinanceiro::ErroPayload, false));
    }
    None
}

/// Classify baixa 422 message body (manual §7.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Baixa422Acao {
    ConvergirPago,
    ConvergirCancelado,
    ManterSolicitadaOuIndeterminada,
    RestaurarAnteriorComErro,
    ErroOperacional,
}

pub fn classificar_baixa_422(mensagem: &str) -> Baixa422Acao {
    let m = mensagem.to_lowercase();
    if m.contains("já liquidado") || m.contains("ja liquidado") {
        Baixa422Acao::ConvergirPago
    } else if m.contains("já baixado") || m.contains("ja baixado") {
        Baixa422Acao::ConvergirCancelado
    } else if m.contains("em processamento") || m.contains("aguarde alguns") {
        Baixa422Acao::ManterSolicitadaOuIndeterminada
    } else if m.contains("aguardando confirmação")
        || m.contains("aguardando confirmacao")
        || m.contains("rejeitado")
        || m.contains("negativação")
        || m.contains("negativacao")
        || m.contains("protesto")
    {
        Baixa422Acao::RestaurarAnteriorComErro
    } else {
        Baixa422Acao::ErroOperacional
    }
}

/// May restore `status_anterior` after a definitive baixa error?
pub fn pode_restaurar_status_anterior(
    atual: StatusFinanceiro,
    anterior: StatusFinanceiro,
) -> bool {
    if atual.is_absolutely_terminal() || anterior.is_absolutely_terminal() {
        return false;
    }
    matches!(
        atual,
        StatusFinanceiro::BaixaEnviando | StatusFinanceiro::AguardandoRetry
    ) && !anterior.is_user_command_terminal()
}

/// Startup orphan recovery targets.
pub fn orphan_startup_target(atual: StatusFinanceiro) -> Option<StatusFinanceiro> {
    match atual {
        StatusFinanceiro::Registrando => Some(StatusFinanceiro::RegistroIndeterminado),
        StatusFinanceiro::BaixaEnviando => Some(StatusFinanceiro::BaixaIndeterminada),
        _ => None,
    }
}

/// Tolerant hasNext parser (manual table: String; JSON examples: boolean).
pub fn parse_has_next(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::String(s) => match s.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" | "sim" => Some(true),
            "false" | "0" | "no" | "nao" | "não" => Some(false),
            _ => None,
        },
        serde_json::Value::Number(n) => n.as_i64().map(|i| i != 0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pago_is_absolute() {
        assert!(!pode_aplicar(
            StatusFinanceiro::Pago,
            StatusFinanceiro::BaixaSolicitada,
            false
        ));
        assert!(!pode_aplicar(
            StatusFinanceiro::Pago,
            StatusFinanceiro::Cancelado,
            true
        ));
    }

    #[test]
    fn cancelado_to_pago_only_with_bank_evidence() {
        assert!(!pode_aplicar(
            StatusFinanceiro::Cancelado,
            StatusFinanceiro::Pago,
            false
        ));
        assert!(pode_aplicar(
            StatusFinanceiro::Cancelado,
            StatusFinanceiro::Pago,
            true
        ));
        assert!(!pode_aplicar(
            StatusFinanceiro::Cancelado,
            StatusFinanceiro::Registrado,
            false
        ));
    }

    #[test]
    fn patch_202_does_not_override_pago() {
        let r = apos_cas_zero(
            StatusFinanceiro::Pago,
            StatusFinanceiro::BaixaSolicitada,
            false,
        );
        assert_eq!(
            r,
            TransicaoResultado::BloqueadaPorPrecedencia {
                atual: StatusFinanceiro::Pago,
                tentou: StatusFinanceiro::BaixaSolicitada,
            }
        );
    }

    #[test]
    fn restore_never_over_pago() {
        assert!(!pode_restaurar_status_anterior(
            StatusFinanceiro::BaixaEnviando,
            StatusFinanceiro::Pago
        ));
        assert!(pode_restaurar_status_anterior(
            StatusFinanceiro::BaixaEnviando,
            StatusFinanceiro::Registrado
        ));
    }

    #[test]
    fn single_em_carteira_does_not_release_patch() {
        assert!(!em_carteira_libera_novo_patch(1, false));
        assert!(!em_carteira_pode_liberar_patch(1, false));
        assert!(!em_carteira_pode_liberar_patch(3, false));
        assert!(em_carteira_pode_liberar_patch(3, true));
    }

    #[test]
    fn classifica_baixa_422() {
        assert_eq!(
            classificar_baixa_422("Operação não permitida: Título já liquidado."),
            Baixa422Acao::ConvergirPago
        );
        assert_eq!(
            classificar_baixa_422("Operação não permitida: Título já baixado."),
            Baixa422Acao::ConvergirCancelado
        );
        assert_eq!(
            classificar_baixa_422(
                "Sua solicitação anterior está em processamento, aguarde alguns instantes."
            ),
            Baixa422Acao::ManterSolicitadaOuIndeterminada
        );
        assert_eq!(
            classificar_baixa_422("Operação não permitida: Título em fluxo de negativação ou protesto."),
            Baixa422Acao::RestaurarAnteriorComErro
        );
    }

    #[test]
    fn orphan_startup() {
        assert_eq!(
            orphan_startup_target(StatusFinanceiro::Registrando),
            Some(StatusFinanceiro::RegistroIndeterminado)
        );
        assert_eq!(
            orphan_startup_target(StatusFinanceiro::BaixaEnviando),
            Some(StatusFinanceiro::BaixaIndeterminada)
        );
        assert_eq!(orphan_startup_target(StatusFinanceiro::Registrado), None);
    }

    #[test]
    fn has_next_bool_and_string() {
        assert_eq!(parse_has_next(&json!(false)), Some(false));
        assert_eq!(parse_has_next(&json!(true)), Some(true));
        assert_eq!(parse_has_next(&json!("false")), Some(false));
        assert_eq!(parse_has_next(&json!("true")), Some(true));
        assert_eq!(parse_has_next(&json!("FALSE")), Some(false));
    }

    #[test]
    fn liquidacao_situacao() {
        let (st, bank) = status_de_situacao_sicredi("LIQUIDADO PIX").unwrap();
        assert_eq!(st, StatusFinanceiro::Pago);
        assert!(bank);
    }
}
