//! AutoBO → Sicredi payload adapter and validation.
//!
//! - No implicit juros/multa defaults
//! - Address: logradouro + ", " + numero; refuse if > 40 (never truncate)
//! - CEP must be 8 digits
//! - Message field name (`mensagem` vs `mensagens`) comes from config after contract test

use rust_decimal::Decimal;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct PagadorSicrediInput {
    pub tipo_pessoa: String, // PF | PJ
    pub documento: String,
    pub nome: String,
    pub logradouro: String,
    pub numero: String,
    pub cidade: String,
    pub uf: String,
    pub cep: String,
    pub telefone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BoletoSicrediInput {
    pub codigo_beneficiario: String,
    pub data_vencimento: String, // YYYY-MM-DD
    pub valor: Decimal,
    pub especie_documento: String,
    pub sicredi_seu_numero: String,
    pub sicredi_id_titulo_empresa: String,
    pub pagador: PagadorSicrediInput,
    pub mensagem_livre: Option<String>,
    /// JSON key after contract test: "mensagem" or "mensagens". Empty = omit.
    pub campo_mensagem_json: Option<String>,
    pub tipo_juros: Option<String>,
    pub percentual_juros_mes: Option<Decimal>,
    pub tipo_multa: Option<String>,
    pub percentual_multa: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptadorErro(pub String);

impl std::fmt::Display for AdaptadorErro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for AdaptadorErro {}

pub fn digits_only(value: &str) -> String {
    value.chars().filter(|c| c.is_ascii_digit()).collect()
}

pub fn montar_endereco(logradouro: &str, numero: &str) -> Result<String, AdaptadorErro> {
    let log = logradouro.trim();
    let num = numero.trim();
    if log.is_empty() {
        return Err(AdaptadorErro(
            "Logradouro do pagador é obrigatório".into(),
        ));
    }
    if num.is_empty() {
        return Err(AdaptadorErro("Número do endereço é obrigatório".into()));
    }
    let endereco = format!("{log}, {num}");
    if endereco.chars().count() > 40 {
        return Err(AdaptadorErro(format!(
            "Endereço com {} caracteres (máx. 40). Abrevie o logradouro — não truncamos automaticamente.",
            endereco.chars().count()
        )));
    }
    Ok(endereco)
}

pub fn validar_cep(cep: &str) -> Result<String, AdaptadorErro> {
    let d = digits_only(cep);
    if d.len() != 8 {
        return Err(AdaptadorErro(
            "CEP deve ter exatamente 8 dígitos".into(),
        ));
    }
    Ok(d)
}

pub fn tipo_pessoa_sicredi(tipo: &str) -> Result<&'static str, AdaptadorErro> {
    match tipo.trim().to_uppercase().as_str() {
        "PF" | "PESSOA_FISICA" => Ok("PESSOA_FISICA"),
        "PJ" | "PESSOA_JURIDICA" => Ok("PESSOA_JURIDICA"),
        other => Err(AdaptadorErro(format!("tipo_pessoa inválido: {other}"))),
    }
}

/// Split free text into up to 4 lines of 80 chars (API message list).
pub fn fatiar_mensagem(texto: &str) -> Vec<String> {
    let t = texto.trim();
    if t.is_empty() {
        return vec![];
    }
    let chars: Vec<char> = t.chars().collect();
    chars
        .chunks(80)
        .take(4)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

pub fn build_cadastro_body(input: &BoletoSicrediInput) -> Result<Value, AdaptadorErro> {
    if input.sicredi_seu_numero.chars().count() > 10 || input.sicredi_seu_numero.is_empty() {
        return Err(AdaptadorErro(
            "sicredi_seu_numero deve ter 1–10 caracteres".into(),
        ));
    }
    if input.sicredi_id_titulo_empresa.chars().count() > 25
        || input.sicredi_id_titulo_empresa.is_empty()
    {
        return Err(AdaptadorErro(
            "sicredi_id_titulo_empresa deve ter 1–25 caracteres".into(),
        ));
    }
    if input.valor <= Decimal::ZERO {
        return Err(AdaptadorErro("Valor deve ser maior que zero".into()));
    }

    let cep = validar_cep(&input.pagador.cep)?;
    let endereco = montar_endereco(&input.pagador.logradouro, &input.pagador.numero)?;
    let cidade = input.pagador.cidade.trim();
    let uf = input.pagador.uf.trim().to_uppercase();
    if cidade.is_empty() {
        return Err(AdaptadorErro("Cidade do pagador é obrigatória".into()));
    }
    if uf.len() != 2 {
        return Err(AdaptadorErro("UF do pagador deve ter 2 letras".into()));
    }
    let tipo = tipo_pessoa_sicredi(&input.pagador.tipo_pessoa)?;
    let doc = digits_only(&input.pagador.documento);
    if doc.len() != 11 && doc.len() != 14 {
        return Err(AdaptadorErro("Documento do pagador inválido".into()));
    }
    let nome = {
        let n = input.pagador.nome.trim();
        if n.is_empty() {
            return Err(AdaptadorErro("Nome do pagador é obrigatório".into()));
        }
        n.to_string()
    };

    let mut body = json!({
        "tipoCobranca": "NORMAL",
        "codigoBeneficiario": input.codigo_beneficiario,
        "dataVencimento": input.data_vencimento,
        "especieDocumento": input.especie_documento,
        "seuNumero": input.sicredi_seu_numero,
        "idTituloEmpresa": input.sicredi_id_titulo_empresa,
        "valor": input.valor,
        "pagador": {
            "tipoPessoa": tipo,
            "documento": doc,
            "nome": nome,
            "endereco": endereco,
            "cidade": cidade,
            "uf": uf,
            "cep": cep,
        }
    });

    if let Some(tel) = &input.pagador.telefone {
        let t = digits_only(tel);
        if !t.is_empty() {
            body["pagador"]["telefone"] = json!(t);
        }
    }
    if let Some(email) = &input.pagador.email {
        let e = email.trim();
        if !e.is_empty() {
            body["pagador"]["email"] = json!(e);
        }
    }

    // Encargos only if explicitly provided
    if let (Some(tj), Some(pj)) = (&input.tipo_juros, input.percentual_juros_mes) {
        if pj > Decimal::ZERO {
            let tipo_j = match tj.to_uppercase().as_str() {
                "PERCENTUAL" | "PERCENTUAL_MES" | "B" => "PERCENTUAL",
                "VALOR" | "A" => "VALOR",
                other => {
                    return Err(AdaptadorErro(format!("tipo_juros inválido: {other}")));
                }
            };
            body["tipoJuros"] = json!(tipo_j);
            if tipo_j == "PERCENTUAL" {
                body["tipoJurosPercentual"] = json!("MENSAL");
            }
            body["juros"] = json!(pj);
        }
    }
    if let (Some(tm), Some(pm)) = (&input.tipo_multa, input.percentual_multa) {
        if pm > Decimal::ZERO {
            let tipo_m = match tm.to_uppercase().as_str() {
                "PERCENTUAL" | "B" => "PERCENTUAL",
                "VALOR" | "A" => "VALOR",
                other => {
                    return Err(AdaptadorErro(format!("tipo_multa inválido: {other}")));
                }
            };
            body["tipoMulta"] = json!(tipo_m);
            body["multa"] = json!(pm);
        }
    }

    if let Some(msg) = &input.mensagem_livre {
        let linhas = fatiar_mensagem(msg);
        if !linhas.is_empty() {
            match input.campo_mensagem_json.as_deref() {
                Some("mensagem") => {
                    body["mensagem"] = json!(linhas);
                }
                Some("mensagens") => {
                    body["mensagens"] = json!(linhas);
                }
                Some(other) if !other.is_empty() => {
                    return Err(AdaptadorErro(format!(
                        "campo_mensagem_json inválido: {other}"
                    )));
                }
                _ => {
                    // Contract test not done — omit rather than guess
                }
            }
        }
    }

    Ok(body)
}

/// Generate sicredi_seu_numero ≤ 10 from local id.
pub fn gerar_sicredi_seu_numero(boleto_id: i64) -> String {
    format!("B{boleto_id:09}")
}

/// Generate idTituloEmpresa ≤ 25 with installation prefix.
pub fn gerar_id_titulo_empresa(prefixo: &str, boleto_id: i64) -> String {
    let p: String = prefixo
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    let p = if p.is_empty() { "ABO".to_string() } else { p };
    let id = format!("{p}-{boleto_id}");
    if id.len() <= 25 {
        id
    } else {
        format!("{p}-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
    }
}

#[derive(Debug)]
pub struct CadastroProbePayload {
    pub with_mensagem: Value,
    pub with_mensagens: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> BoletoSicrediInput {
        BoletoSicrediInput {
            codigo_beneficiario: "12345".into(),
            data_vencimento: "2026-10-01".into(),
            valor: Decimal::new(10050, 2),
            especie_documento: "DUPLICATA_MERCANTIL_INDICACAO".into(),
            sicredi_seu_numero: "B000000001".into(),
            sicredi_id_titulo_empresa: "ABC-1".into(),
            pagador: PagadorSicrediInput {
                tipo_pessoa: "PF".into(),
                documento: "02738306006".into(),
                nome: "RODRIGO".into(),
                logradouro: "RUA DOUTOR VARGAS".into(),
                numero: "150".into(),
                cidade: "PORTO ALEGRE".into(),
                uf: "RS".into(),
                cep: "91250000".into(),
                telefone: None,
                email: None,
            },
            mensagem_livre: None,
            campo_mensagem_json: Some("mensagens".into()),
            tipo_juros: None,
            percentual_juros_mes: None,
            tipo_multa: None,
            percentual_multa: None,
        }
    }

    #[test]
    fn recusa_endereco_longo() {
        let long = "A".repeat(38);
        let err = montar_endereco(&long, "123").unwrap_err();
        assert!(err.0.contains("40"));
    }

    #[test]
    fn endereco_ok() {
        assert_eq!(
            montar_endereco("RUA DOUTOR VARGAS", "150").unwrap(),
            "RUA DOUTOR VARGAS, 150"
        );
    }

    #[test]
    fn sem_encargos_no_body() {
        let body = build_cadastro_body(&base_input()).unwrap();
        assert!(body.get("tipoJuros").is_none());
        assert!(body.get("multa").is_none());
        assert_eq!(body["tipoCobranca"], "NORMAL");
    }

    #[test]
    fn cep_invalido() {
        let mut i = base_input();
        i.pagador.cep = "123".into();
        assert!(build_cadastro_body(&i).is_err());
    }
}
