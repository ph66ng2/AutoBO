import { useEffect, useState } from "react";
import { FileText, Loader2, Receipt, Search, User } from "lucide-react";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Badge } from "../ui/badge";
import { abrirPdfBoleto, gerarBoleto, listarClientesAutoos } from "../../lib/db";
import { validarEntradaBoleto } from "../../lib/boleto-entrada";
import { OPCOES_TIPO_NOTA, especieInicial } from "../../lib/especie-nota";
import {
  consultarCnpj,
  dadosDoCliente,
  mensagemConsultaCnpj,
  preencherInformados,
  preencherVazios,
  validarCNPJ,
  type DadosEmpresa,
} from "../../lib/cnpj";
import { formatCurrency } from "../../lib/format";
import type { BoletoInput, ClienteAutoOS, NFeDados } from "../../types";

interface BoletoRevisaoFormProps {
  modo: "nfe" | "manual";
  dadosNFe?: NFeDados;
  avisos?: string[];
  onVoltar: () => void;
  onSucesso?: () => void;
}

function digits(value: string): string {
  return value.replace(/\D/g, "");
}

function vencimentoPadrao(): string {
  const d = new Date();
  d.setDate(d.getDate() + 30);
  return d.toISOString().split("T")[0];
}

const selectClass =
  "flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm text-foreground";

export function BoletoRevisaoForm({ modo, dadosNFe, avisos, onVoltar, onSucesso }: BoletoRevisaoFormProps) {
  const dest = dadosNFe?.destinatario;
  const item0 = dadosNFe?.itens?.[0];

  const [nome, setNome] = useState(dest?.nome || dest?.razao_social || "");
  const [cpfCnpj, setCpfCnpj] = useState(dest?.documento || "");
  const [email, setEmail] = useState(dest?.email || "");
  const [telefone, setTelefone] = useState(dest?.telefone || "");
  const [logradouro, setLogradouro] = useState(dest?.logradouro || "");
  const [numeroEndereco, setNumeroEndereco] = useState(dest?.numero || "");
  const [bairro, setBairro] = useState(dest?.bairro || "");
  const [cidade, setCidade] = useState(dest?.cidade || "");
  const [uf, setUf] = useState(dest?.uf || "");
  const [cep, setCep] = useState(dest?.cep || "");

  const [numeroNf, setNumeroNf] = useState(dadosNFe?.numero_nf || "");
  const [especieDocumento, setEspecieDocumento] = useState(
    especieInicial(dadosNFe?.serie, dadosNFe?.natureza_operacao),
  );
  const [dataEmissao, setDataEmissao] = useState(dadosNFe?.data_emissao?.slice(0, 10) || "");
  const [valor, setValor] = useState(
    dadosNFe?.valor_total && dadosNFe.valor_total > 0 ? String(dadosNFe.valor_total) : "",
  );
  const [descricao, setDescricao] = useState(item0?.descricao || "");

  const [vencimento, setVencimento] = useState(vencimentoPadrao);
  const [tipoCobranca, setTipoCobranca] = useState("SIMPLES");
  const [mensagem, setMensagem] = useState(item0?.descricao || "");
  const [gerando, setGerando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const [pdfPath, setPdfPath] = useState<string | null>(null);
  const [sugestoes, setSugestoes] = useState<ClienteAutoOS[]>([]);
  const [sugestoesAbertas, setSugestoesAbertas] = useState(false);
  const [consultandoCnpj, setConsultandoCnpj] = useState(false);
  const [avisoCnpj, setAvisoCnpj] = useState<string | null>(null);
  const [termoAplicado, setTermoAplicado] = useState("");

  const termoDocumento = cpfCnpj.trim();
  const termoNome = nome.trim();
  const termoSugestao = modo === "manual"
    ? termoDocumento.length >= 2
      ? termoDocumento
      : termoNome.length >= 2
        ? termoNome
        : ""
    : "";
  const ancoraSugestao = termoDocumento.length >= 2 ? "documento" : "nome";
  const cnpjConsultavel = modo === "manual" && validarCNPJ(cpfCnpj);

  useEffect(() => {
    if (!termoSugestao || termoSugestao === termoAplicado) {
      if (!termoSugestao) setSugestoes([]);
      return;
    }
    const timer = setTimeout(() => {
      listarClientesAutoos(termoSugestao)
        .then((lista) => {
          setSugestoes(lista.slice(0, 8));
          setSugestoesAbertas(true);
        })
        .catch(() => setSugestoes([]));
    }, 200);
    return () => clearTimeout(timer);
  }, [termoSugestao, termoAplicado]);

  function aplicarEmpresa(dados: DadosEmpresa, somenteVazios: boolean) {
    const atual: DadosEmpresa = {
      nome,
      documento: cpfCnpj.replace(/\D/g, ""),
      email,
      telefone,
      cep,
      logradouro,
      numero: numeroEndereco,
      bairro,
      cidade,
      uf,
    };
    const proximo = somenteVazios ? preencherVazios(atual, dados) : preencherInformados(atual, dados);
    setNome(proximo.nome);
    setCpfCnpj(proximo.documento);
    setEmail(proximo.email);
    setTelefone(proximo.telefone);
    setCep(proximo.cep);
    setLogradouro(proximo.logradouro);
    setNumeroEndereco(proximo.numero);
    setBairro(proximo.bairro);
    setCidade(proximo.cidade);
    setUf(proximo.uf);
    setTermoAplicado(proximo.documento.length >= 2 ? proximo.documento : proximo.nome);
    setSugestoesAbertas(false);
  }

  function escolherCliente(cliente: ClienteAutoOS) {
    aplicarEmpresa(dadosDoCliente(cliente), false);
    setAvisoCnpj(null);
  }

  async function buscarDadosCnpj() {
    if (!cnpjConsultavel || consultandoCnpj) return;
    setConsultandoCnpj(true);
    setAvisoCnpj(null);
    setErro(null);
    try {
      const consulta = await consultarCnpj(cpfCnpj);
      aplicarEmpresa(consulta, true);
      setAvisoCnpj("Dados do CNPJ foram preenchidos. Revise antes de gerar o boleto.");
    } catch (error) {
      setAvisoCnpj(mensagemConsultaCnpj(error));
    } finally {
      setConsultandoCnpj(false);
    }
  }

  function listaClientes() {
    if (!sugestoesAbertas || sugestoes.length === 0) return null;
    return (
      <div
        role="listbox"
        aria-label="Clientes da oficina"
        className="absolute z-50 mt-1 max-h-56 w-full overflow-y-auto rounded-md border bg-popover shadow-md"
      >
        {sugestoes.map((cliente) => {
          const dados = dadosDoCliente(cliente);
          return (
            <button
              key={cliente.id}
              type="button"
              role="option"
              className="flex w-full flex-col items-start gap-0.5 px-3 py-2 text-left text-sm hover:bg-accent"
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => escolherCliente(cliente)}
            >
              <span className="font-medium">{dados.nome || "Cliente sem nome"}</span>
              <span className="text-xs text-muted-foreground">
                {[dados.documento, dados.cidade && dados.uf ? `${dados.cidade}/${dados.uf}` : dados.cidade]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
            </button>
          );
        })}
      </div>
    );
  }

  const handleGerar = async () => {
    setGerando(true);
    setErro(null);
    setPdfPath(null);
    try {
      const erroValidacao = validarEntradaBoleto({
        nome,
        documento: cpfCnpj,
        valor,
        vencimento,
        logradouro,
        numero: numeroEndereco,
        cidade,
        uf,
        cep,
      });
      if (erroValidacao) {
        setErro(erroValidacao);
        return;
      }
      const documento = digits(cpfCnpj);
      const valorFinal = parseFloat(valor.replace(",", ".")) || 0;

      const nNota = numeroNf.trim();
      const descricaoFinal = descricao.trim() || mensagem.trim() || "Cobrança";
      const origem = modo === "nfe" ? "NFE" : "MANUAL";

      const dados: BoletoInput = {
        origem,
        seu_numero: nNota || undefined,
        valor_nominal: valorFinal,
        data_vencimento: vencimento,
        tipo_cobranca: tipoCobranca,
        especie_documento: especieDocumento,
        mensagem: mensagem.trim() || descricaoFinal,
        pagador: {
          tipo_pessoa: documento.length > 11 ? "PJ" : "PF",
          documento,
          nome: nome.trim(),
          razao_social: nome.trim(),
          email: email.trim() || undefined,
          telefone: telefone.trim() || undefined,
          cep: digits(cep) || undefined,
          logradouro: logradouro.trim() || undefined,
          numero: numeroEndereco.trim() || undefined,
          bairro: bairro.trim() || undefined,
          cidade: cidade.trim() || undefined,
          uf: uf.trim() || undefined,
        },
        itens: [
          {
            descricao: descricaoFinal,
            quantidade: 1,
            valor_unitario: valorFinal,
            subtotal: valorFinal,
          },
        ],
      };

      if (origem === "NFE") {
        const emissao = dataEmissao || new Date().toISOString().slice(0, 10);
        dados.nfe = {
          numero_nf: nNota || dadosNFe?.numero_nf || "S_N",
          serie: dadosNFe?.serie,
          chave_acesso: dadosNFe?.chave_acesso || `MANUAL-${nNota || Date.now()}`,
          data_emissao: emissao,
          valor_total: valorFinal,
          natureza_operacao: dadosNFe?.natureza_operacao,
        };
      }

      const resultado = await gerarBoleto(dados);
      setPdfPath(resultado.pdf_path);
      await abrirPdfBoleto(resultado.pdf_path).catch(() => undefined);
    } catch (e) {
      setErro(String(e));
    } finally {
      setGerando(false);
    }
  };

  return (
    <form
      className="space-y-6"
      onSubmit={(event) => {
        event.preventDefault();
        void handleGerar();
      }}
    >
          {modo === "nfe" && avisos && avisos.length > 0 && (
            <div className="space-y-1 rounded-md border border-amber-400 bg-amber-100 p-3 text-sm text-amber-900">
              {avisos.map((aviso, i) => (
                <p key={i}>{aviso}</p>
              ))}
            </div>
          )}

          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <User className="h-4 w-4" />
              <h3 className="text-sm font-semibold">Dados do pagador</h3>
              <Badge variant={modo === "nfe" ? "default" : "warning"}>
                {modo === "nfe" ? "Nota / PDF" : "Manual"}
              </Badge>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="relative col-span-2 space-y-2">
                <Label>Nome / razão social *</Label>
                <Input
                  placeholder="Nome do pagador"
                  value={nome}
                  onChange={(e) => {
                    setNome(e.target.value);
                    setSugestoesAbertas(true);
                  }}
                  onFocus={() => setSugestoesAbertas(true)}
                />
                {ancoraSugestao === "nome" && listaClientes()}
              </div>
              <div className="relative space-y-2">
                <Label>CPF / CNPJ *</Label>
                <div className="flex gap-2">
                  <Input
                    placeholder="CPF ou CNPJ"
                    value={cpfCnpj}
                    onChange={(e) => {
                      setCpfCnpj(e.target.value);
                      setAvisoCnpj(null);
                      setSugestoesAbertas(true);
                    }}
                    onFocus={() => setSugestoesAbertas(true)}
                  />
                  {modo === "manual" && (
                    <Button
                      type="button"
                      variant="outline"
                      size="icon"
                      className="shrink-0"
                      onClick={() => void buscarDadosCnpj()}
                      disabled={!cnpjConsultavel || consultandoCnpj}
                      aria-label="Buscar dados do CNPJ"
                      title="Buscar dados do CNPJ"
                    >
                      {consultandoCnpj ? (
                        <Loader2 className="animate-spin" aria-hidden="true" />
                      ) : (
                        <Search aria-hidden="true" />
                      )}
                    </Button>
                  )}
                </div>
                {ancoraSugestao === "documento" && listaClientes()}
                {avisoCnpj && (
                  <p className="mt-1 text-xs text-muted-foreground">{avisoCnpj}</p>
                )}
              </div>
              <div className="space-y-2">
                <Label>E-mail</Label>
                <Input
                  placeholder="email@cliente.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>Telefone</Label>
                <Input
                  placeholder="(71) 00000-0000"
                  value={telefone}
                  onChange={(e) => setTelefone(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>CEP</Label>
                <Input
                  placeholder="00000-000"
                  value={cep}
                  onChange={(e) => setCep(e.target.value)}
                />
              </div>
              <div className="col-span-2 space-y-2">
                <Label>Logradouro</Label>
                <Input
                  placeholder="Rua, avenida..."
                  value={logradouro}
                  onChange={(e) => setLogradouro(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>Número</Label>
                <Input
                  value={numeroEndereco}
                  onChange={(e) => setNumeroEndereco(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>Bairro</Label>
                <Input
                  value={bairro}
                  onChange={(e) => setBairro(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>Cidade</Label>
                <Input
                  value={cidade}
                  onChange={(e) => setCidade(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>UF</Label>
                <Input
                  maxLength={2}
                  placeholder="BA"
                  value={uf}
                  onChange={(e) => setUf(e.target.value.toUpperCase())}
                />
              </div>
            </div>
          </div>

          <hr />
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <FileText className="h-4 w-4" />
              <h3 className="text-sm font-semibold">Nota e cobrança</h3>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Nº da nota (documento do boleto) *</Label>
                <Input
                  placeholder="447"
                  value={numeroNf}
                  onChange={(e) => setNumeroNf(e.target.value)}
                />
                <p className="mt-1 text-xs text-muted-foreground">Vai no campo documento / seu número do boleto.</p>
              </div>
              <div className="space-y-2">
                <Label>Tipo da nota</Label>
                <select
                  className={selectClass}
                  value={especieDocumento}
                  onChange={(e) => setEspecieDocumento(e.target.value)}
                >
                  {OPCOES_TIPO_NOTA.map((opcao) => (
                    <option key={opcao.value} value={opcao.value}>
                      {opcao.label}
                    </option>
                  ))}
                </select>
                <p className="mt-1 text-xs text-muted-foreground">
                  {dadosNFe?.serie && !dadosNFe.serie.toLowerCase().includes("nfs")
                    ? `Série fiscal da nota: ${dadosNFe.serie}. `
                    : ""}
                  No Sicredi isso vira a espécie do título.
                </p>
              </div>
              <div className="space-y-2">
                <Label>Emissão</Label>
                <Input
                  type="date"
                  value={dataEmissao}
                  onChange={(e) => setDataEmissao(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>Valor (R$) *</Label>
                <Input
                  type="number"
                  step="0.01"
                  min={0}
                  placeholder="0,00"
                  value={valor}
                  onChange={(e) => setValor(e.target.value)}
                />
              </div>
              <div className="col-span-2 space-y-2">
                <Label>Descrição do serviço</Label>
                <Input
                  placeholder="O que está sendo cobrado"
                  value={descricao}
                  onChange={(e) => {
                    setDescricao(e.target.value);
                    if (!mensagem || mensagem === descricao) {
                      setMensagem(e.target.value);
                    }
                  }}
                />
              </div>
            </div>
            {valor && parseFloat(valor.replace(",", ".")) > 0 && (
              <p className="text-sm text-muted-foreground">
                Total: <span className="font-semibold tabular-nums text-foreground">{formatCurrency(parseFloat(valor.replace(",", ".")) || 0)}</span>
              </p>
            )}
          </div>

          <hr />
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <Receipt className="h-4 w-4" />
              <h3 className="text-sm font-semibold">Boleto</h3>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Vencimento *</Label>
                <Input
                  type="date"
                  value={vencimento}
                  onChange={(e) => setVencimento(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>Tipo de cobrança</Label>
                <select
                  className={selectClass}
                  value={tipoCobranca}
                  onChange={(e) => setTipoCobranca(e.target.value)}
                >
                  <option value="SIMPLES">Simples</option>
                  <option value="RECORRENTE">Recorrente</option>
                  <option value="UNICA">Única</option>
                </select>
              </div>
              <div className="col-span-2 space-y-2">
                <Label>Mensagem no boleto</Label>
                <Input
                  placeholder="Mensagem que aparece no PDF"
                  value={mensagem}
                  onChange={(e) => setMensagem(e.target.value)}
                />
              </div>
            </div>
          </div>

          {erro && (
            <div className="rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">
              {erro}
            </div>
          )}
          {pdfPath && (
            <div className="space-y-2 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-800">
              <p>PDF salvo em {pdfPath}</p>
              <Button size="sm" variant="outline" type="button" onClick={() => onSucesso?.()}>
                Concluir
              </Button>
            </div>
          )}

          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <Button variant="outline" type="button" onClick={onVoltar}>
              Cancelar
            </Button>
            <Button type="submit" disabled={gerando}>
              {gerando ? "Gerando..." : "Gerar boleto"}
            </Button>
          </div>
    </form>
  );
}
