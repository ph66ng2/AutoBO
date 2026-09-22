use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};

use crate::validators::{cnpj, cpf};

#[derive(Debug, Serialize)]
pub struct NFeDados {
    pub numero_nf: String,
    pub serie: Option<String>,
    pub chave_acesso: String,
    pub data_emissao: String,
    pub valor_total: f64,
    pub valor_produtos: Option<f64>,
    pub natureza_operacao: Option<String>,
    pub destinatario: DestinatarioDados,
    pub itens: Vec<NFeItem>,
    pub versao: String,
}

#[derive(Debug, Serialize)]
pub struct DestinatarioDados {
    pub documento: String,
    pub nome: Option<String>,
    pub razao_social: Option<String>,
    pub email: Option<String>,
    pub telefone: Option<String>,
    pub cep: Option<String>,
    pub logradouro: Option<String>,
    pub numero: Option<String>,
    pub complemento: Option<String>,
    pub bairro: Option<String>,
    pub cidade: Option<String>,
    pub uf: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NFeItem {
    pub descricao: String,
    pub quantidade: f64,
    pub unidade: String,
    pub valor_unitario: f64,
    pub subtotal: f64,
    pub ncm: Option<String>,
}

#[derive(Debug)]
pub struct NFeError(String);

impl std::fmt::Display for NFeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NFeError: {}", self.0)
    }
}

impl std::error::Error for NFeError {}

#[derive(Debug, Deserialize)]
struct NfeProcWrapper {
    #[serde(rename = "NFe")]
    nfe: NfeRoot,
}

#[derive(Debug, Deserialize)]
struct NfeRoot {
    #[serde(rename = "infNFe")]
    inf_nfe: InfNFe,
}

#[derive(Debug, Deserialize)]
struct InfNFe {
    #[serde(rename = "@Id")]
    id: String,
    #[serde(rename = "@versao")]
    versao: String,
    ide: Ide,
    dest: Option<Dest>,
    #[serde(rename = "det", default)]
    det: Vec<Det>,
    total: Total,
}

#[derive(Debug, Deserialize)]
struct Ide {
    #[serde(rename = "nNF")]
    n_nf: String,
    serie: Option<String>,
    #[serde(rename = "dhEmi")]
    dh_emi: Option<String>,
    #[serde(rename = "dEmi")]
    d_emi: Option<String>,
    #[serde(rename = "natOp")]
    nat_op: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Dest {
    #[serde(rename = "CNPJ")]
    cnpj: Option<String>,
    #[serde(rename = "CPF")]
    cpf: Option<String>,
    #[serde(rename = "xNome")]
    x_nome: Option<String>,
    #[serde(rename = "enderDest")]
    ender_dest: Option<EnderDest>,
}

#[derive(Debug, Deserialize, Default)]
struct EnderDest {
    #[serde(rename = "xLgr")]
    x_lgr: Option<String>,
    #[serde(rename = "nro")]
    nro: Option<String>,
    #[serde(rename = "xCpl")]
    x_cpl: Option<String>,
    #[serde(rename = "xBairro")]
    x_bairro: Option<String>,
    #[serde(rename = "xMun")]
    x_mun: Option<String>,
    #[serde(rename = "UF")]
    uf: Option<String>,
    #[serde(rename = "CEP")]
    cep: Option<String>,
    #[serde(rename = "fone")]
    fone: Option<String>,
    #[serde(rename = "email")]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Det {
    #[serde(rename = "@nItem")]
    _n_item: Option<String>,
    prod: Prod,
}

#[derive(Debug, Deserialize)]
struct Prod {
    #[serde(rename = "xProd")]
    x_prod: String,
    #[serde(rename = "qCom")]
    q_com: f64,
    #[serde(rename = "uCom")]
    u_com: String,
    #[serde(rename = "vUnCom")]
    v_un_com: f64,
    #[serde(rename = "vProd")]
    v_prod: f64,
    #[serde(rename = "NCM")]
    ncm: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Total {
    #[serde(rename = "ICMSTot")]
    icms_tot: IcmsTot,
}

#[derive(Debug, Deserialize)]
struct IcmsTot {
    #[serde(rename = "vNF")]
    v_nf: f64,
    #[serde(rename = "vProd")]
    v_prod: Option<f64>,
}

pub fn parse_nfe_xml(xml: &str) -> Result<NFeDados, NFeError> {
    let xml = xml.strip_prefix('\u{FEFF}').unwrap_or(xml);

    let inf_nfe = match from_str::<NfeProcWrapper>(xml) {
        Ok(wrapper) => wrapper.nfe.inf_nfe,
        Err(_) => from_str::<NfeRoot>(xml)
            .map_err(|e| NFeError(format!("Failed to parse NFe XML: {}", e)))?
            .inf_nfe,
    };

    let chave_acesso = if inf_nfe.id.starts_with("NFe") {
        inf_nfe.id[3..].to_string()
    } else {
        inf_nfe.id.clone()
    };

    if chave_acesso.len() != 44 || !chave_acesso.chars().all(|c| c.is_ascii_digit()) {
        return Err(NFeError(format!(
            "Invalid chave de acesso: must be 44 digits, got {}",
            chave_acesso.len()
        )));
    }

    let versao = match inf_nfe.versao.as_str() {
        "3.10" => "3.10".to_string(),
        "4.00" => "4.00".to_string(),
        _ => {
            return Err(NFeError(format!(
                "Unsupported NFe version: {}",
                inf_nfe.versao
            )))
        }
    };

    let data_emissao = if let Some(dh_emi) = &inf_nfe.ide.dh_emi {
        dh_emi.split('T').next().unwrap_or(dh_emi).to_string()
    } else if let Some(d_emi) = &inf_nfe.ide.d_emi {
        d_emi.clone()
    } else {
        return Err(NFeError("Missing emission date (dhEmi or dEmi)".into()));
    };

    let emissao_date = chrono::NaiveDate::parse_from_str(&data_emissao, "%Y-%m-%d")
        .map_err(|e| NFeError(format!("Invalid emission date format: {}", e)))?;
    let today = chrono::Local::now().naive_local().date();
    if emissao_date > today {
        return Err(NFeError("Emission date is in the future".into()));
    }

    let valor_total = inf_nfe.total.icms_tot.v_nf;
    if valor_total <= 0.0 {
        return Err(NFeError(format!(
            "Invalid total value: must be > 0, got {}",
            valor_total
        )));
    }

    if inf_nfe.det.is_empty() {
        return Err(NFeError("NFe must have at least 1 item".into()));
    }

    let dest = inf_nfe
        .dest
        .ok_or_else(|| NFeError("Missing destinatario".into()))?;
    let documento = dest
        .cnpj
        .or(dest.cpf)
        .ok_or_else(|| NFeError("Missing CNPJ/CPF in destinatario".into()))?;

    let doc_clean: String = documento.chars().filter(|c| c.is_ascii_digit()).collect();
    let doc_len = doc_clean.len();

    if doc_len == 11 {
        if !cpf::validar_cpf(&doc_clean) {
            return Err(NFeError("Invalid CPF in destinatario".into()));
        }
    } else if doc_len == 14 {
        if !cnpj::validar_cnpj(&doc_clean) {
            return Err(NFeError("Invalid CNPJ in destinatario".into()));
        }
    } else {
        return Err(NFeError(format!(
            "Invalid document length: expected 11 (CPF) or 14 (CNPJ), got {}",
            doc_len
        )));
    }

    let ender = dest.ender_dest.unwrap_or_default();
    let x_nome = dest.x_nome.unwrap_or_default();

    let (nome, razao_social) = if doc_len == 11 {
        (Some(x_nome), None)
    } else {
        (None, Some(x_nome))
    };

    let itens: Vec<NFeItem> = inf_nfe
        .det
        .into_iter()
        .map(|det| NFeItem {
            descricao: det.prod.x_prod,
            quantidade: det.prod.q_com,
            unidade: det.prod.u_com,
            valor_unitario: det.prod.v_un_com,
            subtotal: det.prod.v_prod,
            ncm: det.prod.ncm,
        })
        .collect();

    let valor_produtos = inf_nfe.total.icms_tot.v_prod;
    if let Some(declarado) = valor_produtos {
        let soma_itens: f64 = itens.iter().map(|item| item.subtotal).sum();
        if (declarado - soma_itens).abs() > 0.01 {
            return Err(NFeError(format!(
                "Inconsistent total: item sum {soma_itens} differs from vProd {declarado}"
            )));
        }
    }

    Ok(NFeDados {
        numero_nf: inf_nfe.ide.n_nf,
        serie: inf_nfe.ide.serie,
        chave_acesso,
        data_emissao,
        valor_total,
        valor_produtos,
        natureza_operacao: inf_nfe.ide.nat_op,
        destinatario: DestinatarioDados {
            documento: doc_clean,
            nome,
            razao_social,
            email: ender.email,
            telefone: ender.fone,
            cep: ender.cep,
            logradouro: ender.x_lgr,
            numero: ender.nro,
            complemento: ender.x_cpl,
            bairro: ender.x_bairro,
            cidade: ender.x_mun,
            uf: ender.uf,
        },
        itens,
        versao,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_nfe_v40_cnpj() -> &'static str {
        r#"<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe Id="NFe35123456789012345678901234567890123456789012" versao="4.00">
    <ide>
      <cUF>35</cUF>
      <nNF>123</nNF>
      <serie>1</serie>
      <dhEmi>2024-01-15T10:00:00-03:00</dhEmi>
      <natOp>Venda</natOp>
    </ide>
    <dest>
      <CNPJ>11222333000181</CNPJ>
      <xNome>Empresa Teste LTDA</xNome>
      <enderDest>
        <xLgr>Rua A</xLgr>
        <nro>123</nro>
        <xCpl>Apto 1</xCpl>
        <xBairro>Centro</xBairro>
        <xMun>Sao Paulo</xMun>
        <UF>SP</UF>
        <CEP>01001000</CEP>
        <fone>11999999999</fone>
        <email>test@example.com</email>
      </enderDest>
    </dest>
    <det nItem="1">
      <prod>
        <xProd>Produto A</xProd>
        <qCom>2.00</qCom>
        <uCom>UN</uCom>
        <vUnCom>10.00</vUnCom>
        <vProd>20.00</vProd>
        <NCM>12345678</NCM>
      </prod>
    </det>
    <total>
      <ICMSTot>
        <vNF>20.00</vNF>
        <vProd>20.00</vProd>
      </ICMSTot>
    </total>
  </infNFe>
</NFe>"#
    }

    fn sample_nfe_v310_cpf() -> &'static str {
        r#"<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe Id="NFe35123456789012345678901234567890123456789012" versao="3.10">
    <ide>
      <cUF>35</cUF>
      <nNF>456</nNF>
      <serie>2</serie>
      <dEmi>2024-01-15</dEmi>
    </ide>
    <dest>
      <CPF>52998224725</CPF>
      <xNome>Joao Silva</xNome>
    </dest>
    <det nItem="1">
      <prod>
        <xProd>Produto B</xProd>
        <qCom>1.00</qCom>
        <uCom>UN</uCom>
        <vUnCom>10.00</vUnCom>
        <vProd>10.00</vProd>
      </prod>
    </det>
    <total>
      <ICMSTot>
        <vNF>10.00</vNF>
      </ICMSTot>
    </total>
  </infNFe>
</NFe>"#
    }

    fn sample_nfe_proc_v40() -> &'static str {
        r#"<nfeProc xmlns="http://www.portalfiscal.inf.br/nfe" versao="4.00">
  <NFe>
    <infNFe Id="NFe35123456789012345678901234567890123456789012" versao="4.00">
      <ide>
        <cUF>35</cUF>
        <nNF>789</nNF>
        <serie>3</serie>
        <dhEmi>2024-01-15T10:00:00-03:00</dhEmi>
      </ide>
      <dest>
        <CNPJ>11222333000181</CNPJ>
        <xNome>Empresa Teste LTDA</xNome>
      </dest>
      <det nItem="1">
        <prod>
          <xProd>Produto C</xProd>
          <qCom>1.00</qCom>
          <uCom>UN</uCom>
          <vUnCom>5.00</vUnCom>
          <vProd>5.00</vProd>
        </prod>
      </det>
      <total>
        <ICMSTot>
          <vNF>5.00</vNF>
        </ICMSTot>
      </total>
    </infNFe>
  </NFe>
</nfeProc>"#
    }

    #[test]
    fn test_parse_nfe_v40_cnpj() {
        let result = parse_nfe_xml(sample_nfe_v40_cnpj()).expect("parse failed");
        assert_eq!(result.numero_nf, "123");
        assert_eq!(result.serie, Some("1".to_string()));
        assert_eq!(result.chave_acesso, "35123456789012345678901234567890123456789012");
        assert_eq!(result.data_emissao, "2024-01-15");
        assert_eq!(result.valor_total, 20.0);
        assert_eq!(result.valor_produtos, Some(20.0));
        assert_eq!(result.natureza_operacao, Some("Venda".to_string()));
        assert_eq!(result.versao, "4.00");
        assert_eq!(result.destinatario.documento, "11222333000181");
        assert_eq!(result.destinatario.razao_social, Some("Empresa Teste LTDA".to_string()));
        assert_eq!(result.destinatario.nome, None);
        assert_eq!(result.destinatario.email, Some("test@example.com".to_string()));
        assert_eq!(result.destinatario.telefone, Some("11999999999".to_string()));
        assert_eq!(result.destinatario.cep, Some("01001000".to_string()));
        assert_eq!(result.destinatario.logradouro, Some("Rua A".to_string()));
        assert_eq!(result.destinatario.numero, Some("123".to_string()));
        assert_eq!(result.destinatario.complemento, Some("Apto 1".to_string()));
        assert_eq!(result.destinatario.bairro, Some("Centro".to_string()));
        assert_eq!(result.destinatario.cidade, Some("Sao Paulo".to_string()));
        assert_eq!(result.destinatario.uf, Some("SP".to_string()));
        assert_eq!(result.itens.len(), 1);
        assert_eq!(result.itens[0].descricao, "Produto A");
        assert_eq!(result.itens[0].quantidade, 2.0);
        assert_eq!(result.itens[0].unidade, "UN");
        assert_eq!(result.itens[0].valor_unitario, 10.0);
        assert_eq!(result.itens[0].subtotal, 20.0);
        assert_eq!(result.itens[0].ncm, Some("12345678".to_string()));
    }

    #[test]
    fn test_parse_nfe_v310_cpf() {
        let result = parse_nfe_xml(sample_nfe_v310_cpf()).expect("parse failed");
        assert_eq!(result.numero_nf, "456");
        assert_eq!(result.serie, Some("2".to_string()));
        assert_eq!(result.chave_acesso, "35123456789012345678901234567890123456789012");
        assert_eq!(result.data_emissao, "2024-01-15");
        assert_eq!(result.valor_total, 10.0);
        assert_eq!(result.versao, "3.10");
        assert_eq!(result.destinatario.documento, "52998224725");
        assert_eq!(result.destinatario.nome, Some("Joao Silva".to_string()));
        assert_eq!(result.destinatario.razao_social, None);
        assert_eq!(result.itens.len(), 1);
        assert_eq!(result.itens[0].descricao, "Produto B");
    }

    #[test]
    fn test_parse_nfe_proc_wrapper() {
        let result = parse_nfe_xml(sample_nfe_proc_v40()).expect("parse failed");
        assert_eq!(result.numero_nf, "789");
        assert_eq!(result.versao, "4.00");
        assert_eq!(result.destinatario.documento, "11222333000181");
    }

    #[test]
    fn test_invalid_chave_acesso() {
        let xml = r#"<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe Id="NFe123" versao="4.00">
    <ide><nNF>1</nNF><dhEmi>2024-01-15T10:00:00-03:00</dhEmi></ide>
    <dest><CNPJ>11222333000181</CNPJ><xNome>Teste</xNome></dest>
    <det nItem="1"><prod><xProd>P</xProd><qCom>1</qCom><uCom>UN</uCom><vUnCom>1</vUnCom><vProd>1</vProd></prod></det>
    <total><ICMSTot><vNF>1</vNF></ICMSTot></total>
  </infNFe>
</NFe>"#;
        let result = parse_nfe_xml(xml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("chave de acesso"));
    }

    #[test]
    fn test_total_inconsistente_nao_produz_dados() {
        let xml = r#"<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe Id="NFe35123456789012345678901234567890123456789012" versao="4.00">
    <ide><nNF>1</nNF><dhEmi>2024-01-15T10:00:00-03:00</dhEmi></ide>
    <dest><CNPJ>11222333000181</CNPJ><xNome>Teste</xNome></dest>
    <det nItem="1"><prod><xProd>P</xProd><qCom>1</qCom><uCom>UN</uCom><vUnCom>10</vUnCom><vProd>10</vProd></prod></det>
    <total><ICMSTot><vNF>10.00</vNF><vProd>99.00</vProd></ICMSTot></total>
  </infNFe>
</NFe>"#;
        let err = parse_nfe_xml(xml).unwrap_err().to_string();
        assert!(err.contains("Inconsistent total"));
    }

    #[test]
    fn test_invalid_valor_total() {
        let xml = r#"<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe Id="NFe35123456789012345678901234567890123456789012" versao="4.00">
    <ide><nNF>1</nNF><dhEmi>2024-01-15T10:00:00-03:00</dhEmi></ide>
    <dest><CNPJ>11222333000181</CNPJ><xNome>Teste</xNome></dest>
    <det nItem="1"><prod><xProd>P</xProd><qCom>1</qCom><uCom>UN</uCom><vUnCom>1</vUnCom><vProd>1</vProd></prod></det>
    <total><ICMSTot><vNF>0.00</vNF></ICMSTot></total>
  </infNFe>
</NFe>"#;
        let result = parse_nfe_xml(xml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("total value"));
    }

    #[test]
    fn test_no_items() {
        let xml = r#"<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe Id="NFe35123456789012345678901234567890123456789012" versao="4.00">
    <ide><nNF>1</nNF><dhEmi>2024-01-15T10:00:00-03:00</dhEmi></ide>
    <dest><CNPJ>11222333000181</CNPJ><xNome>Teste</xNome></dest>
    <total><ICMSTot><vNF>1</vNF></ICMSTot></total>
  </infNFe>
</NFe>"#;
        let result = parse_nfe_xml(xml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("at least 1 item"));
    }

    #[test]
    fn test_invalid_cpf_dest() {
        let xml = r#"<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe Id="NFe35123456789012345678901234567890123456789012" versao="4.00">
    <ide><nNF>1</nNF><dhEmi>2024-01-15T10:00:00-03:00</dhEmi></ide>
    <dest><CPF>12345678901</CPF><xNome>Teste</xNome></dest>
    <det nItem="1"><prod><xProd>P</xProd><qCom>1</qCom><uCom>UN</uCom><vUnCom>1</vUnCom><vProd>1</vProd></prod></det>
    <total><ICMSTot><vNF>1</vNF></ICMSTot></total>
  </infNFe>
</NFe>"#;
        let result = parse_nfe_xml(xml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("CPF"));
    }

    #[test]
    fn test_unsupported_version() {
        let xml = r#"<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe Id="NFe35123456789012345678901234567890123456789012" versao="2.00">
    <ide><nNF>1</nNF><dhEmi>2024-01-15T10:00:00-03:00</dhEmi></ide>
    <dest><CNPJ>11222333000181</CNPJ><xNome>Teste</xNome></dest>
    <det nItem="1"><prod><xProd>P</xProd><qCom>1</qCom><uCom>UN</uCom><vUnCom>1</vUnCom><vProd>1</vProd></prod></det>
    <total><ICMSTot><vNF>1</vNF></ICMSTot></total>
  </infNFe>
</NFe>"#;
        let result = parse_nfe_xml(xml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("version"));
    }
}
