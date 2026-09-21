import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Badge } from "../ui/badge";
import { abrirPdfBoleto, gerarBoleto } from "../../lib/db";
import { formatCurrency } from "../../lib/format";
import type { BoletoInput, NFeDados } from "../../types";

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
  "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground";

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
  const [serie, setSerie] = useState(dadosNFe?.serie || "");
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

  const handleGerar = async () => {
    setGerando(true);
    setErro(null);
    setPdfPath(null);
    try {
      const documento = digits(cpfCnpj);
      if (!documento) {
        setErro("Informe o CPF/CNPJ do pagador.");
        return;
      }
      if (!nome.trim()) {
        setErro("Informe o nome do pagador.");
        return;
      }
      const valorFinal = parseFloat(valor.replace(",", ".")) || 0;
      if (!(valorFinal > 0)) {
        setErro("Informe o valor do boleto.");
        return;
      }
      if (!vencimento) {
        setErro("Informe o vencimento.");
        return;
      }
      if (!logradouro.trim() || !numeroEndereco.trim()) {
        setErro("Informe logradouro e número do pagador (obrigatório para Sicredi).");
        return;
      }
      if (!cidade.trim() || uf.trim().length !== 2) {
        setErro("Informe cidade e UF (2 letras) do pagador.");
        return;
      }
      if (digits(cep).length !== 8) {
        setErro("CEP deve ter 8 dígitos.");
        return;
      }
      const enderecoMontado = `${logradouro.trim()}, ${numeroEndereco.trim()}`;
      if (enderecoMontado.length > 40) {
        setErro(
          `Endereço com ${enderecoMontado.length} caracteres (máx. 40). Abrevie o logradouro.`,
        );
        return;
      }

      const nNota = numeroNf.trim();
      const descricaoFinal = descricao.trim() || mensagem.trim() || "Cobrança";
      const origem = modo === "nfe" ? "NFE" : "MANUAL";

      const dados: BoletoInput = {
        origem,
        seu_numero: nNota || undefined,
        valor_nominal: valorFinal,
        data_vencimento: vencimento,
        tipo_cobranca: tipoCobranca,
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
          serie: serie.trim() || dadosNFe?.serie,
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
    <div className="space-y-4">
      <Button variant="outline" size="sm" onClick={onVoltar}>
        ← Voltar
      </Button>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <CardTitle>
              {modo === "nfe" ? "Revisar e editar boleto" : "Novo boleto — entrada manual"}
            </CardTitle>
            <Badge variant={modo === "nfe" ? "default" : "warning"}>
              {modo === "nfe" ? "Nota / PDF" : "Manual"}
            </Badge>
          </div>
          <p className="mt-2 text-sm text-muted-foreground">
            Tudo aqui é editável. O Nº da nota vira o documento do boleto.
          </p>
        </CardHeader>
        <CardContent className="space-y-6">
          {modo === "nfe" && avisos && avisos.length > 0 && (
            <div className="space-y-1 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
              {avisos.map((aviso, i) => (
                <p key={i}>⚠ {aviso}</p>
              ))}
            </div>
          )}

          <div className="space-y-3">
            <h3 className="text-sm font-medium text-muted-foreground">Pagador</h3>
            <div className="grid grid-cols-2 gap-3">
              <div className="col-span-2">
                <Label>Nome / razão social *</Label>
                <Input
                  className="mt-1"
                  placeholder="Nome do pagador"
                  value={nome}
                  onChange={(e) => setNome(e.target.value)}
                />
              </div>
              <div>
                <Label>CPF / CNPJ *</Label>
                <Input
                  className="mt-1"
                  placeholder="000.000.000-00"
                  value={cpfCnpj}
                  onChange={(e) => setCpfCnpj(e.target.value)}
                />
              </div>
              <div>
                <Label>E-mail</Label>
                <Input
                  className="mt-1"
                  placeholder="email@cliente.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </div>
              <div>
                <Label>Telefone</Label>
                <Input
                  className="mt-1"
                  placeholder="(71) 00000-0000"
                  value={telefone}
                  onChange={(e) => setTelefone(e.target.value)}
                />
              </div>
              <div>
                <Label>CEP</Label>
                <Input
                  className="mt-1"
                  placeholder="00000-000"
                  value={cep}
                  onChange={(e) => setCep(e.target.value)}
                />
              </div>
              <div className="col-span-2">
                <Label>Logradouro</Label>
                <Input
                  className="mt-1"
                  placeholder="Rua, avenida..."
                  value={logradouro}
                  onChange={(e) => setLogradouro(e.target.value)}
                />
              </div>
              <div>
                <Label>Número</Label>
                <Input
                  className="mt-1"
                  value={numeroEndereco}
                  onChange={(e) => setNumeroEndereco(e.target.value)}
                />
              </div>
              <div>
                <Label>Bairro</Label>
                <Input
                  className="mt-1"
                  value={bairro}
                  onChange={(e) => setBairro(e.target.value)}
                />
              </div>
              <div>
                <Label>Cidade</Label>
                <Input
                  className="mt-1"
                  value={cidade}
                  onChange={(e) => setCidade(e.target.value)}
                />
              </div>
              <div>
                <Label>UF</Label>
                <Input
                  className="mt-1"
                  maxLength={2}
                  placeholder="BA"
                  value={uf}
                  onChange={(e) => setUf(e.target.value.toUpperCase())}
                />
              </div>
            </div>
          </div>

          <div className="space-y-3 border-t pt-4">
            <h3 className="text-sm font-medium text-muted-foreground">Nota e cobrança</h3>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label>Nº da nota (documento do boleto) *</Label>
                <Input
                  className="mt-1"
                  placeholder="447"
                  value={numeroNf}
                  onChange={(e) => setNumeroNf(e.target.value)}
                />
                <p className="mt-1 text-xs text-muted-foreground">Vai no campo documento / seu número do boleto.</p>
              </div>
              <div>
                <Label>Série</Label>
                <Input
                  className="mt-1"
                  placeholder="NFS-e ou 1"
                  value={serie}
                  onChange={(e) => setSerie(e.target.value)}
                />
              </div>
              <div>
                <Label>Emissão</Label>
                <Input
                  className="mt-1"
                  type="date"
                  value={dataEmissao}
                  onChange={(e) => setDataEmissao(e.target.value)}
                />
              </div>
              <div>
                <Label>Valor (R$) *</Label>
                <Input
                  className="mt-1"
                  type="number"
                  step="0.01"
                  min={0}
                  placeholder="0,00"
                  value={valor}
                  onChange={(e) => setValor(e.target.value)}
                />
              </div>
              <div className="col-span-2">
                <Label>Descrição do serviço</Label>
                <Input
                  className="mt-1"
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

          <div className="space-y-3 border-t pt-4">
            <h3 className="text-sm font-medium text-muted-foreground">Boleto</h3>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label>Vencimento *</Label>
                <Input
                  className="mt-1"
                  type="date"
                  value={vencimento}
                  onChange={(e) => setVencimento(e.target.value)}
                />
              </div>
              <div>
                <Label>Tipo de cobrança</Label>
                <select
                  className={`${selectClass} mt-1`}
                  value={tipoCobranca}
                  onChange={(e) => setTipoCobranca(e.target.value)}
                >
                  <option value="SIMPLES">Simples</option>
                  <option value="RECORRENTE">Recorrente</option>
                  <option value="UNICA">Única</option>
                </select>
              </div>
              <div className="col-span-2">
                <Label>Mensagem no boleto</Label>
                <Input
                  className="mt-1"
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
              <Button size="sm" variant="outline" onClick={() => onSucesso?.()}>
                Concluir
              </Button>
            </div>
          )}

          <div className="flex justify-end gap-2 pt-2">
            <Button variant="outline" onClick={onVoltar}>
              Cancelar
            </Button>
            <Button onClick={() => void handleGerar()} disabled={gerando}>
              {gerando ? "Gerando..." : "Gerar boleto"}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
