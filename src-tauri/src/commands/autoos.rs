//! AutoOS integration — shared Postgres tables.
//!
//! - `clientes` / `equipamentos`: read-only
//! - `produtos` + `movimentacoes_estoque`: write with the same movement rule as AutoOS
//!   (atomic increment/decrement + trail). Quantity is never overwritten from a form.
//!
//! If AutoOS tables are missing, list commands return empty results and mutations
//! return a clear error. Boleto flows do not depend on these tables.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::{
    clientes_disponivel, coluna_existe, equipamentos_disponivel, produtos_disponivel, tabela_existe,
    AppState,
};
use crate::services::oficina_pdf::{
    gerar_pdf_oficina, parse_linhas_orcamento, status_label, DocumentoOficinaDados,
    DocumentoOficinaGerado, TipoDocumentoOficina,
};
use sqlx::QueryBuilder;

const PRODUTO_SELECT: &str = r#"
    SELECT id, codigo, nome, descricao, categoria,
           quantidade_estoque, quantidade_minima, quantidade_maxima,
           unidade_medida, localizacao,
           preco_custo::FLOAT8 as preco_custo,
           preco_venda::FLOAT8 as preco_venda,
           margem_lucro::FLOAT8 as margem_lucro,
           marca_original, tipo_cartucho, cor, rendimento, modelos_compativeis,
           fornecedor_principal, prazo_entrega, ativo,
           criado_em::TEXT as criado_em, atualizado_em::TEXT as atualizado_em
    FROM produtos
"#;

#[derive(Debug, Serialize)]
pub struct IntegracaoAutoOSDto {
    pub clientes: bool,
    pub equipamentos: bool,
    pub produtos: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ClienteAutoOS {
    pub id: i32,
    pub nome: Option<String>,
    pub tipo_pessoa: Option<String>,
    pub documento: Option<String>,
    pub razao_social: Option<String>,
    pub nome_fantasia: Option<String>,
    pub cpf_cnpj: Option<String>,
    pub telefone: Option<String>,
    pub email: Option<String>,
    pub cep: Option<String>,
    pub endereco: Option<String>,
    pub numero: Option<String>,
    pub complemento: Option<String>,
    pub bairro: Option<String>,
    pub cidade: Option<String>,
    pub uf: Option<String>,
    pub ativo: Option<bool>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct EquipamentoAutoOS {
    pub id: i32,
    pub serial_number: String,
    pub marca: String,
    pub modelo: String,
    pub tipo: String,
    pub status: Option<String>,
    pub data_entrada: String,
    pub cliente_id: Option<i32>,
    pub cliente_nome: Option<String>,
    pub cliente_documento: Option<String>,
    pub cliente_telefone: Option<String>,
    pub cliente_email: Option<String>,
    pub responsavel_nome: Option<String>,
    pub responsavel_email: Option<String>,
    pub responsavel_telefone: Option<String>,
    pub valor_orcamento: Option<f64>,
    pub valor_final: Option<f64>,
    pub data_pronto: Option<String>,
    pub data_saida: Option<String>,
    pub prazo_aprovacao: Option<String>,
    pub data_aprovacao: Option<String>,
    pub data_verificacao: Option<String>,
    pub defeito_relatado: Option<String>,
    pub acessorios: Option<String>,
    pub acessorios_outros: Option<String>,
    pub observacoes: Option<String>,
    pub patrimonio: Option<String>,
    pub tecnologia: Option<String>,
    pub conectividade: Option<String>,
    pub paginas_impressas: Option<i32>,
    pub proprietario: Option<String>,
    pub criado_em: Option<String>,
    pub data_reprovacao: Option<String>,
    pub tem_verificacao: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct VerificacaoAutoOS {
    pub id: i32,
    pub equipamento_id: i32,
    pub tecnico_nome: Option<String>,
    pub problema_relatado: Option<String>,
    pub diagnostico: Option<String>,
    pub servicos_necessarios: Option<String>,
    pub pecas_necessarias: Option<String>,
    pub custo_estimado_mao_obra: Option<f64>,
    pub custo_estimado_pecas: Option<f64>,
    pub custo_total: Option<f64>,
    pub observacoes: Option<String>,
    pub forma_pagamento_codigo: Option<String>,
    pub forma_pagamento_detalhe: Option<String>,
    pub adjusted_at: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ProdutoAutoOS {
    pub id: i32,
    pub codigo: String,
    pub nome: String,
    pub descricao: Option<String>,
    pub categoria: String,
    pub quantidade_estoque: Option<i32>,
    pub quantidade_minima: Option<i32>,
    pub quantidade_maxima: Option<i32>,
    pub unidade_medida: Option<String>,
    pub localizacao: Option<String>,
    pub preco_custo: Option<f64>,
    pub preco_venda: Option<f64>,
    pub margem_lucro: Option<f64>,
    pub marca_original: Option<String>,
    pub tipo_cartucho: Option<String>,
    pub cor: Option<String>,
    pub rendimento: Option<i32>,
    pub modelos_compativeis: Option<String>,
    pub fornecedor_principal: Option<String>,
    pub prazo_entrega: Option<i32>,
    pub ativo: Option<bool>,
    pub criado_em: Option<String>,
    pub atualizado_em: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MovimentacaoEstoqueRow {
    pub id: i32,
    pub produto_id: i32,
    pub tipo: String,
    pub quantidade: i32,
    pub origem: String,
    pub referencia: Option<String>,
    pub data_hora: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ProdutoCadastroInput {
    pub codigo: String,
    pub nome: String,
    pub descricao: Option<String>,
    pub categoria: String,
    pub quantidade_inicial: Option<i32>,
    pub quantidade_minima: Option<i32>,
    pub quantidade_maxima: Option<i32>,
    pub unidade_medida: Option<String>,
    pub localizacao: Option<String>,
    pub preco_custo: f64,
    pub preco_venda: f64,
    pub margem_lucro: Option<f64>,
    pub marca_original: Option<String>,
    pub tipo_cartucho: Option<String>,
    pub cor: Option<String>,
    pub rendimento: Option<i32>,
    pub modelos_compativeis: Option<String>,
    pub fornecedor_principal: Option<String>,
    pub prazo_entrega: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct MovimentacaoEstoqueInput {
    pub produto_id: i32,
    pub tipo: String,
    pub quantidade: i32,
    pub origem: String,
    pub referencia: Option<String>,
}

fn required_text(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} é obrigatório"));
    }
    Ok(trimmed.to_string())
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

pub(crate) fn validate_movimentacao_input(
    input: &MovimentacaoEstoqueInput,
) -> Result<(String, String, Option<String>), String> {
    if input.quantidade <= 0 {
        return Err("Quantidade da movimentação deve ser maior que zero".into());
    }

    let movimento = input.tipo.trim().to_uppercase();
    if movimento != "ENTRADA" && movimento != "SAIDA" {
        return Err("Tipo de movimentação inválido".into());
    }

    let origem = input.origem.trim().to_string();
    if origem.is_empty() {
        return Err("Origem da movimentação é obrigatória".into());
    }

    let referencia = input
        .referencia
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());

    Ok((movimento, origem, referencia))
}

pub(crate) fn calculate_resulting_stock(
    quantidade_atual: i32,
    quantidade: i32,
    movimento: &str,
) -> Result<i32, String> {
    let quantidade_resultante = if movimento == "ENTRADA" {
        quantidade_atual + quantidade
    } else {
        quantidade_atual - quantidade
    };

    if quantidade_resultante < 0 {
        return Err("Estoque insuficiente para registrar a saída informada".into());
    }

    Ok(quantidade_resultante)
}

async fn require_produtos(state: &AppState) -> Result<(), String> {
    if produtos_disponivel(&state.db).await {
        Ok(())
    } else {
        Err("Estoque AutoOS indisponível neste banco (modo standalone)".into())
    }
}

fn text_or_dash(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("—")
        .to_string()
}

async fn equipamento_select_sql(state: &AppState) -> String {
    let com_verificacoes = tabela_existe(&state.db, "verificacoes").await;
    let valor = if com_verificacoes {
        r#"COALESCE(
               valor_orcamento,
               (
                   SELECT v.custo_total
                   FROM verificacoes v
                   WHERE v.equipamento_id = equipamentos.id
                   ORDER BY v.id DESC
                   LIMIT 1
               )
           )::FLOAT8 as valor_orcamento"#
    } else {
        "valor_orcamento::FLOAT8 as valor_orcamento"
    };
    let tem_verificacao = if com_verificacoes {
        "EXISTS(SELECT 1 FROM verificacoes v WHERE v.equipamento_id = equipamentos.id) as tem_verificacao"
    } else {
        "false as tem_verificacao"
    };
    let cliente_documento = if clientes_disponivel(&state.db).await {
        let doc_expr = if coluna_existe(&state.db, "clientes", "cpf_cnpj").await {
            "COALESCE(c.documento, c.cpf_cnpj)"
        } else {
            "c.documento"
        };
        format!(
            "(SELECT {doc_expr} FROM clientes c WHERE c.id = equipamentos.cliente_id LIMIT 1) AS cliente_documento"
        )
    } else {
        "NULL::TEXT AS cliente_documento".to_string()
    };
    let data_reprovacao = if coluna_existe(&state.db, "equipamentos", "data_reprovacao").await {
        "data_reprovacao"
    } else {
        "NULL::TEXT as data_reprovacao"
    };
    let proprietario = if coluna_existe(&state.db, "equipamentos", "proprietario").await {
        "proprietario"
    } else {
        "NULL::TEXT as proprietario"
    };
    let criado_em = if coluna_existe(&state.db, "equipamentos", "criado_em").await {
        "criado_em::TEXT as criado_em"
    } else {
        "NULL::TEXT as criado_em"
    };

    format!(
        r#"SELECT id, serial_number, marca, modelo, tipo, status, data_entrada,
                  cliente_id, cliente_nome, {cliente_documento},
                  cliente_telefone, cliente_email,
                  responsavel_nome, responsavel_email, responsavel_telefone,
                  {valor},
                  valor_final::FLOAT8 as valor_final,
                  data_pronto, data_saida, prazo_aprovacao, data_aprovacao, data_verificacao,
                  {data_reprovacao}, defeito_relatado, acessorios, acessorios_outros, observacoes,
                  patrimonio, tecnologia, conectividade, paginas_impressas,
                  {proprietario}, {criado_em},
                  {tem_verificacao}
           FROM equipamentos"#
    )
}

async fn verificacao_select_sql(state: &AppState) -> String {
    let forma_codigo = if coluna_existe(&state.db, "verificacoes", "forma_pagamento_codigo").await {
        "forma_pagamento_codigo"
    } else {
        "NULL::TEXT as forma_pagamento_codigo"
    };
    let forma_detalhe = if coluna_existe(&state.db, "verificacoes", "forma_pagamento_detalhe").await
    {
        "forma_pagamento_detalhe"
    } else {
        "NULL::TEXT as forma_pagamento_detalhe"
    };
    let adjusted = if coluna_existe(&state.db, "verificacoes", "adjusted_at").await {
        "adjusted_at::TEXT as adjusted_at"
    } else {
        "NULL::TEXT as adjusted_at"
    };
    format!(
        r#"SELECT id, equipamento_id, tecnico_nome, problema_relatado, diagnostico,
                  servicos_necessarios, pecas_necessarias,
                  custo_estimado_mao_obra::FLOAT8 as custo_estimado_mao_obra,
                  custo_estimado_pecas::FLOAT8 as custo_estimado_pecas,
                  custo_total::FLOAT8 as custo_total,
                  observacoes, {forma_codigo}, {forma_detalhe}, {adjusted}
           FROM verificacoes"#
    )
}

async fn buscar_produto(state: &AppState, id: i32) -> Result<ProdutoAutoOS, String> {
    let query = format!("{PRODUTO_SELECT} WHERE id = $1");
    sqlx::query_as::<_, ProdutoAutoOS>(&query)
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| format!("Produto não encontrado: {e}"))
}

#[tauri::command]
pub async fn verificar_integracao_autoos(
    state: tauri::State<'_, AppState>,
) -> Result<IntegracaoAutoOSDto, String> {
    Ok(IntegracaoAutoOSDto {
        clientes: clientes_disponivel(&state.db).await,
        equipamentos: equipamentos_disponivel(&state.db).await,
        produtos: produtos_disponivel(&state.db).await,
    })
}

#[tauri::command]
pub async fn listar_clientes_autoos(
    busca: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ClienteAutoOS>, String> {
    if !clientes_disponivel(&state.db).await {
        return Ok(vec![]);
    }

    let termo = busca
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| format!("%{v}%"));

    if let Some(pattern) = termo {
        sqlx::query_as::<_, ClienteAutoOS>(
            r#"SELECT id, nome, tipo_pessoa, documento, razao_social, nome_fantasia,
                      cpf_cnpj, telefone, email, cep, endereco, numero, complemento,
                      bairro, cidade, uf, ativo
               FROM clientes
               WHERE COALESCE(ativo, true) = true
                 AND (
                   COALESCE(nome, '') ILIKE $1
                   OR COALESCE(razao_social, '') ILIKE $1
                   OR COALESCE(nome_fantasia, '') ILIKE $1
                   OR COALESCE(documento, '') ILIKE $1
                   OR COALESCE(cpf_cnpj, '') ILIKE $1
                 )
               ORDER BY COALESCE(nome, razao_social, '') ASC
               LIMIT 200"#,
        )
        .bind(pattern)
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Erro ao consultar clientes: {e}"))
    } else {
        sqlx::query_as::<_, ClienteAutoOS>(
            r#"SELECT id, nome, tipo_pessoa, documento, razao_social, nome_fantasia,
                      cpf_cnpj, telefone, email, cep, endereco, numero, complemento,
                      bairro, cidade, uf, ativo
               FROM clientes
               WHERE COALESCE(ativo, true) = true
               ORDER BY COALESCE(nome, razao_social, '') ASC
               LIMIT 200"#,
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Erro ao consultar clientes: {e}"))
    }
}

#[tauri::command]
pub async fn listar_equipamentos_autoos(
    busca: Option<String>,
    status: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<EquipamentoAutoOS>, String> {
    if !equipamentos_disponivel(&state.db).await {
        return Ok(vec![]);
    }

    let select = equipamento_select_sql(&state).await;
    let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(select);
    query_builder.push(" WHERE 1=1");

    if let Some(busca) = busca
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let pattern = format!("%{busca}%");
        query_builder.push(" AND (");
        query_builder.push("serial_number ILIKE ");
        query_builder.push_bind(pattern.clone());
        query_builder.push(" OR marca ILIKE ");
        query_builder.push_bind(pattern.clone());
        query_builder.push(" OR modelo ILIKE ");
        query_builder.push_bind(pattern.clone());
        query_builder.push(" OR COALESCE(cliente_nome, '') ILIKE ");
        query_builder.push_bind(pattern.clone());
        query_builder.push(" OR COALESCE(defeito_relatado, '') ILIKE ");
        query_builder.push_bind(pattern);
        query_builder.push(")");
    }

    if let Some(status) = status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "TODOS")
    {
        query_builder.push(" AND status = ");
        query_builder.push_bind(status.to_string());
    }

    query_builder.push(" ORDER BY id DESC LIMIT 200");

    query_builder
        .build_query_as::<EquipamentoAutoOS>()
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Erro ao consultar equipamentos: {e}"))
}

async fn buscar_equipamento_autoos(
    state: &AppState,
    id: i32,
) -> Result<EquipamentoAutoOS, String> {
    let query = format!("{} WHERE id = $1", equipamento_select_sql(state).await);
    sqlx::query_as::<_, EquipamentoAutoOS>(&query)
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| format!("Equipamento não encontrado: {e}"))
}

#[tauri::command]
pub async fn buscar_verificacao_autoos(
    equipamento_id: i32,
    state: tauri::State<'_, AppState>,
) -> Result<Option<VerificacaoAutoOS>, String> {
    if !tabela_existe(&state.db, "verificacoes").await {
        return Ok(None);
    }

    let query = format!(
        "{} WHERE equipamento_id = $1 ORDER BY id DESC LIMIT 1",
        verificacao_select_sql(&state).await
    );
    sqlx::query_as::<_, VerificacaoAutoOS>(&query)
    .bind(equipamento_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Erro ao consultar verificação: {e}"))
}

#[tauri::command]
pub async fn gerar_documento_oficina(
    equipamento_id: i32,
    tipo: String,
    state: tauri::State<'_, AppState>,
) -> Result<DocumentoOficinaGerado, String> {
    if !equipamentos_disponivel(&state.db).await {
        return Err("Equipamentos AutoOS indisponíveis neste banco".into());
    }

    let tipo = TipoDocumentoOficina::parse(&tipo)?;
    let eq = buscar_equipamento_autoos(&state, equipamento_id).await?;
    let verificacao = if tabela_existe(&state.db, "verificacoes").await {
        let query = format!(
            "{} WHERE equipamento_id = $1 ORDER BY id DESC LIMIT 1",
            verificacao_select_sql(&state).await
        );
        sqlx::query_as::<_, VerificacaoAutoOS>(&query)
        .bind(equipamento_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("Erro ao consultar verificação: {e}"))?
    } else {
        None
    };

    let linhas = parse_linhas_orcamento(
        verificacao
            .as_ref()
            .and_then(|row| row.servicos_necessarios.as_deref()),
        verificacao
            .as_ref()
            .and_then(|row| row.pecas_necessarias.as_deref()),
    );
    let soma_linhas: f64 = linhas.iter().map(|linha| linha.valor).sum();
    let total = verificacao
        .as_ref()
        .and_then(|row| row.custo_total)
        .or(eq.valor_orcamento)
        .filter(|value| *value > 0.0)
        .unwrap_or(soma_linhas);

    let contato = [
        eq.responsavel_telefone
            .as_deref()
            .or(eq.cliente_telefone.as_deref()),
        eq.responsavel_email
            .as_deref()
            .or(eq.cliente_email.as_deref()),
    ]
    .into_iter()
    .flatten()
    .filter(|value| !value.trim().is_empty())
    .collect::<Vec<_>>()
    .join(" | ");

    let dados = DocumentoOficinaDados {
        equipamento_id: eq.id,
        serial_number: eq.serial_number.clone(),
        marca: eq.marca.clone(),
        modelo: eq.modelo.clone(),
        tipo: eq.tipo.clone(),
        status_label: status_label(eq.status.as_deref().unwrap_or("")),
        cliente_nome: text_or_dash(eq.cliente_nome.as_deref()),
        cliente_documento: text_or_dash(eq.cliente_documento.as_deref()),
        contato: if contato.is_empty() {
            "—".into()
        } else {
            contato
        },
        responsavel: text_or_dash(
            eq.responsavel_nome
                .as_deref()
                .or(eq.cliente_nome.as_deref()),
        ),
        data_entrada: eq.data_entrada.clone(),
        patrimonio: text_or_dash(eq.patrimonio.as_deref()),
        defeito: text_or_dash(
            eq.defeito_relatado
                .as_deref()
                .or(verificacao
                    .as_ref()
                    .and_then(|row| row.problema_relatado.as_deref())),
        ),
        diagnostico: text_or_dash(
            verificacao
                .as_ref()
                .and_then(|row| row.diagnostico.as_deref()),
        ),
        acessorios: text_or_dash(eq.acessorios.as_deref()),
        acessorios_outros: text_or_dash(eq.acessorios_outros.as_deref()),
        observacoes: text_or_dash(eq.observacoes.as_deref()),
        tecnico: text_or_dash(
            verificacao
                .as_ref()
                .and_then(|row| row.tecnico_nome.as_deref()),
        ),
        linhas,
        total,
    };

    let path = gerar_pdf_oficina(tipo, &dados)?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("documento.pdf")
        .to_string();

    Ok(DocumentoOficinaGerado {
        tipo: tipo.as_str().to_string(),
        filename,
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn listar_produtos_autoos(
    busca: Option<String>,
    categoria: Option<String>,
    apenas_estoque_baixo: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ProdutoAutoOS>, String> {
    if !produtos_disponivel(&state.db).await {
        return Ok(vec![]);
    }

    // Mesmos filtros do AutoOS `listar_produtos`: só ativos, busca, categoria, estoque baixo.
    let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(format!(
        "{PRODUTO_SELECT} WHERE ativo = true"
    ));

    if let Some(busca) = busca
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let pattern = format!("%{busca}%");
        query_builder.push(" AND (");
        query_builder.push("codigo ILIKE ");
        query_builder.push_bind(pattern.clone());
        query_builder.push(" OR nome ILIKE ");
        query_builder.push_bind(pattern.clone());
        query_builder.push(" OR COALESCE(descricao, '') ILIKE ");
        query_builder.push_bind(pattern.clone());
        query_builder.push(" OR COALESCE(marca_original, '') ILIKE ");
        query_builder.push_bind(pattern);
        query_builder.push(")");
    }

    if let Some(categoria) = categoria
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "TODOS")
    {
        query_builder.push(" AND categoria = ");
        query_builder.push_bind(categoria.to_string());
    }

    if apenas_estoque_baixo.unwrap_or(false) {
        query_builder.push(
            " AND COALESCE(quantidade_estoque, 0) < COALESCE(quantidade_minima, 0)",
        );
    }

    query_builder.push(" ORDER BY nome ASC, id DESC");

    query_builder
        .build_query_as::<ProdutoAutoOS>()
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Erro ao consultar produtos: {e}"))
}

#[tauri::command]
pub async fn criar_produto_autoos(
    input: ProdutoCadastroInput,
    state: tauri::State<'_, AppState>,
) -> Result<ProdutoAutoOS, String> {
    require_produtos(&state).await?;

    let codigo = required_text(&input.codigo, "Código")?;
    let nome = required_text(&input.nome, "Nome")?;
    let categoria = required_text(&input.categoria, "Categoria")?;
    let unidade = optional_text(input.unidade_medida.as_deref()).unwrap_or_else(|| "UN".into());
    let qtd_min = input.quantidade_minima.unwrap_or(5);
    let qtd_max = input.quantidade_maxima.unwrap_or(50);
    let qtd_inicial = input.quantidade_inicial.unwrap_or(0);

    if qtd_inicial < 0 {
        return Err("Quantidade inicial não pode ser negativa".into());
    }
    if qtd_min < 0 {
        return Err("Quantidade mínima não pode ser negativa".into());
    }
    if qtd_max < qtd_min {
        return Err("Quantidade máxima deve ser maior ou igual à mínima".into());
    }
    if input.preco_custo < 0.0 || input.preco_venda < 0.0 {
        return Err("Preços não podem ser negativos".into());
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| format!("Erro ao iniciar transação: {e}"))?;

    let row = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO produtos (
            codigo, nome, descricao, categoria,
            quantidade_estoque, quantidade_minima, quantidade_maxima,
            unidade_medida, localizacao, preco_custo, preco_venda, margem_lucro,
            marca_original, tipo_cartucho, cor, rendimento, modelos_compativeis,
            fornecedor_principal, prazo_entrega, empresa_id
        ) VALUES (
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,
            (SELECT id FROM empresas ORDER BY id ASC LIMIT 1)
        ) RETURNING id"#,
    )
    .bind(&codigo)
    .bind(&nome)
    .bind(optional_text(input.descricao.as_deref()))
    .bind(&categoria)
    .bind(qtd_inicial)
    .bind(qtd_min)
    .bind(qtd_max)
    .bind(&unidade)
    .bind(optional_text(input.localizacao.as_deref()))
    .bind(input.preco_custo)
    .bind(input.preco_venda)
    .bind(input.margem_lucro)
    .bind(optional_text(input.marca_original.as_deref()))
    .bind(optional_text(input.tipo_cartucho.as_deref()))
    .bind(optional_text(input.cor.as_deref()))
    .bind(input.rendimento)
    .bind(optional_text(input.modelos_compativeis.as_deref()))
    .bind(optional_text(input.fornecedor_principal.as_deref()))
    .bind(input.prazo_entrega)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Erro ao criar produto: {e}"))?;

    if qtd_inicial > 0 {
        sqlx::query(
            r#"INSERT INTO movimentacoes_estoque (
                produto_id, tipo, quantidade, origem, referencia, usuario, data_hora, empresa_id
            )
            SELECT $1, 'ENTRADA', $2, 'COMPRA', 'CADASTRO', 'Aline', NOW(), empresa_id
            FROM produtos
            WHERE id = $1"#,
        )
        .bind(row)
        .bind(qtd_inicial)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Erro ao registrar saldo inicial: {e}"))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Erro ao confirmar cadastro: {e}"))?;

    buscar_produto(&state, row).await
}

#[tauri::command]
pub async fn atualizar_cadastro_produto_autoos(
    id: i32,
    input: ProdutoCadastroInput,
    state: tauri::State<'_, AppState>,
) -> Result<ProdutoAutoOS, String> {
    require_produtos(&state).await?;

    let codigo = required_text(&input.codigo, "Código")?;
    let nome = required_text(&input.nome, "Nome")?;
    let categoria = required_text(&input.categoria, "Categoria")?;
    let unidade = optional_text(input.unidade_medida.as_deref()).unwrap_or_else(|| "UN".into());
    let qtd_min = input.quantidade_minima.unwrap_or(5);
    let qtd_max = input.quantidade_maxima;

    if qtd_min < 0 {
        return Err("Quantidade mínima não pode ser negativa".into());
    }
    if let Some(max) = qtd_max {
        if max < qtd_min {
            return Err("Quantidades mínima/máxima inválidas".into());
        }
    }
    if input.preco_custo < 0.0 || input.preco_venda < 0.0 {
        return Err("Preços não podem ser negativos".into());
    }

    // Cadastro and price only — never overwrite quantidade_estoque or extra catalog fields.
    let updated = sqlx::query(
        r#"UPDATE produtos SET
            codigo = $1, nome = $2, descricao = $3, categoria = $4,
            quantidade_minima = $5,
            quantidade_maxima = COALESCE($6, quantidade_maxima),
            unidade_medida = COALESCE(NULLIF($7, ''), unidade_medida, 'UN'),
            localizacao = $8, preco_custo = $9, preco_venda = $10,
            atualizado_em = NOW()
        WHERE id = $11 AND COALESCE(ativo, true) = true"#,
    )
    .bind(&codigo)
    .bind(&nome)
    .bind(optional_text(input.descricao.as_deref()))
    .bind(&categoria)
    .bind(qtd_min)
    .bind(input.quantidade_maxima)
    .bind(&unidade)
    .bind(optional_text(input.localizacao.as_deref()))
    .bind(input.preco_custo)
    .bind(input.preco_venda)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Erro ao atualizar produto: {e}"))?;

    if updated.rows_affected() == 0 {
        return Err("Produto não encontrado ou inativo".into());
    }

    buscar_produto(&state, id).await
}

#[tauri::command]
pub async fn registrar_movimentacao_estoque(
    input: MovimentacaoEstoqueInput,
    state: tauri::State<'_, AppState>,
) -> Result<ProdutoAutoOS, String> {
    require_produtos(&state).await?;
    let (movimento, origem, referencia) = validate_movimentacao_input(&input)?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| format!("Erro ao iniciar transação: {e}"))?;

    if movimento == "ENTRADA" {
        sqlx::query_scalar::<_, i32>(
            r#"UPDATE produtos
               SET quantidade_estoque = COALESCE(quantidade_estoque, 0) + $1,
                   atualizado_em = NOW()
               WHERE id = $2 AND COALESCE(ativo, true) = true
               RETURNING quantidade_estoque"#,
        )
        .bind(input.quantidade)
        .bind(input.produto_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Erro ao registrar entrada: {e}"))?
        .ok_or_else(|| "Produto não encontrado".to_string())?;
    } else {
        let atualizado = sqlx::query_scalar::<_, i32>(
            r#"UPDATE produtos
               SET quantidade_estoque = COALESCE(quantidade_estoque, 0) - $1,
                   atualizado_em = NOW()
               WHERE id = $2
                 AND COALESCE(ativo, true) = true
                 AND COALESCE(quantidade_estoque, 0) >= $1
               RETURNING quantidade_estoque"#,
        )
        .bind(input.quantidade)
        .bind(input.produto_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Erro ao registrar saída: {e}"))?;

        if atualizado.is_none() {
            let saldo = sqlx::query_scalar::<_, i32>(
                "SELECT COALESCE(quantidade_estoque, 0) FROM produtos WHERE id = $1 AND COALESCE(ativo, true) = true",
            )
            .bind(input.produto_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Erro ao ler saldo: {e}"))?
            .ok_or_else(|| "Produto não encontrado".to_string())?;
            calculate_resulting_stock(saldo, input.quantidade, &movimento)?;
            return Err("Estoque insuficiente para registrar a saída informada".into());
        }
    }

    sqlx::query(
        r#"INSERT INTO movimentacoes_estoque (
            produto_id, tipo, quantidade, origem, referencia, usuario, data_hora, empresa_id
        )
        SELECT $1, $2, $3, $4, $5, 'Aline', NOW(), empresa_id
        FROM produtos
        WHERE id = $1"#,
    )
    .bind(input.produto_id)
    .bind(&movimento)
    .bind(input.quantidade)
    .bind(&origem)
    .bind(referencia)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Erro ao gravar trilha de estoque: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("Erro ao confirmar movimentação: {e}"))?;

    buscar_produto(&state, input.produto_id).await
}

#[tauri::command]
pub async fn listar_movimentacoes_produto(
    produto_id: i32,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MovimentacaoEstoqueRow>, String> {
    if !produtos_disponivel(&state.db).await {
        return Ok(vec![]);
    }

    sqlx::query_as::<_, MovimentacaoEstoqueRow>(
        r#"SELECT id, produto_id, tipo, quantidade, origem, referencia,
                  data_hora::TEXT as data_hora
           FROM movimentacoes_estoque
           WHERE produto_id = $1
           ORDER BY id DESC
           LIMIT 50"#,
    )
    .bind(produto_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Erro ao listar movimentações: {e}"))
}

#[derive(Debug, Serialize, FromRow)]
pub struct EquipamentoImagemRow {
    pub id: i32,
    pub equipamento_id: i32,
    pub categoria: String,
    pub filename: String,
    pub mime_type: String,
    pub tamanho_bytes: i32,
    pub largura: Option<i32>,
    pub altura: Option<i32>,
    pub ordem: i32,
    pub observacao: Option<String>,
    pub storage_path: String,
    pub criado_em: Option<String>,
    pub atualizado_em: Option<String>,
}

#[tauri::command]
pub async fn listar_imagens_equipamento(
    equipamento_id: i32,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<EquipamentoImagemRow>, String> {
    if !tabela_existe(&state.db, "equipamento_imagens").await {
        return Ok(vec![]);
    }

    sqlx::query_as::<_, EquipamentoImagemRow>(
        r#"SELECT id, equipamento_id, categoria, filename, mime_type,
                  tamanho_bytes, largura, altura, ordem, observacao, storage_path,
                  criado_em::TEXT as criado_em, atualizado_em::TEXT as atualizado_em
           FROM equipamento_imagens
           WHERE equipamento_id = $1
           ORDER BY categoria ASC, ordem ASC, id ASC"#,
    )
    .bind(equipamento_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Erro ao listar imagens: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movement(tipo: &str, quantidade: i32, origem: &str) -> MovimentacaoEstoqueInput {
        MovimentacaoEstoqueInput {
            produto_id: 1,
            tipo: tipo.to_string(),
            quantidade,
            origem: origem.to_string(),
            referencia: None,
        }
    }

    #[test]
    fn rejeita_quantidade_zero() {
        let err = validate_movimentacao_input(&movement("ENTRADA", 0, "COMPRA")).unwrap_err();
        assert!(err.contains("maior que zero"));
    }

    #[test]
    fn rejeita_tipo_invalido() {
        let err = validate_movimentacao_input(&movement("AJUSTE", 2, "COMPRA")).unwrap_err();
        assert!(err.contains("inválido"));
    }

    #[test]
    fn entrada_e_saida_calculam_saldo() {
        assert_eq!(calculate_resulting_stock(10, 3, "ENTRADA").unwrap(), 13);
        assert_eq!(calculate_resulting_stock(10, 3, "SAIDA").unwrap(), 7);
        assert!(calculate_resulting_stock(2, 3, "SAIDA")
            .unwrap_err()
            .contains("insuficiente"));
    }
}
