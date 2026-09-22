export interface EntradaBoleto {
  nome: string;
  documento: string;
  valor: string;
  vencimento: string;
  logradouro: string;
  numero: string;
  cidade: string;
  uf: string;
  cep: string;
}

export interface RascunhoManual {
  id: string;
  seuNumero: string;
  documento: string;
  nome: string;
  valor: number;
  vencimento: string;
  status: "RASCUNHO";
  origem: "MANUAL";
}

export function somenteDigitos(value: string): string {
  return value.replace(/\D/g, "");
}

export function validarEntradaBoleto(entrada: EntradaBoleto): string | null {
  const documento = somenteDigitos(entrada.documento);
  if (!documento) return "Informe o CPF/CNPJ do pagador.";
  if (!entrada.nome.trim()) return "Informe o nome do pagador.";
  const valorFinal = parseFloat(entrada.valor.replace(",", ".")) || 0;
  if (!(valorFinal > 0)) return "Informe o valor do boleto.";
  if (!entrada.vencimento) return "Informe o vencimento.";
  if (!entrada.logradouro.trim() || !entrada.numero.trim()) {
    return "Informe logradouro e número do pagador (obrigatório para Sicredi).";
  }
  if (!entrada.cidade.trim() || entrada.uf.trim().length !== 2) {
    return "Informe cidade e UF (2 letras) do pagador.";
  }
  if (somenteDigitos(entrada.cep).length !== 8) return "CEP deve ter 8 dígitos.";
  const endereco = `${entrada.logradouro.trim()}, ${entrada.numero.trim()}`;
  if (endereco.length > 40) {
    return `Endereço com ${endereco.length} caracteres (máx. 40). Abrevie o logradouro.`;
  }
  return null;
}

export function criarMemoriaBoletosManuais() {
  const porId = new Map<string, RascunhoManual>();
  const porSeuNumero = new Map<string, string>();

  return {
    criar(entrada: EntradaBoleto, seuNumero: string): { ok: true; id: string } | { ok: false; erro: string } {
      const erro = validarEntradaBoleto(entrada);
      if (erro) return { ok: false, erro };
      const existente = porSeuNumero.get(seuNumero);
      if (existente) return { ok: true, id: existente };
      const id = `manual-${porId.size + 1}`;
      const rascunho: RascunhoManual = {
        id,
        seuNumero,
        documento: somenteDigitos(entrada.documento),
        nome: entrada.nome.trim(),
        valor: parseFloat(entrada.valor.replace(",", ".")) || 0,
        vencimento: entrada.vencimento,
        status: "RASCUNHO",
        origem: "MANUAL",
      };
      porId.set(id, rascunho);
      porSeuNumero.set(seuNumero, id);
      return { ok: true, id };
    },
    consultar(id: string): RascunhoManual | null {
      return porId.get(id) ?? null;
    },
    tamanho(): number {
      return porId.size;
    },
  };
}
