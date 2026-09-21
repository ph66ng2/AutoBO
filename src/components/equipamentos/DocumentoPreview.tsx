import { formatCurrency, formatDatePtBr } from "../../lib/format";
import {
  parsePecasOrcamento,
  parseServicosOrcamento,
  totalOrcamento,
} from "../../lib/documentos-oficina";
import { labelStatusEquipamento, numeroOS } from "../../lib/status-equipamento";
import type {
  EquipamentoAutoOS,
  TipoDocumentoOficina,
  VerificacaoAutoOS,
} from "../../types";

const CABECALHO = {
  nome: "BMITAG TECNOLOGIA QRCODE E RFID",
  descricao: "Vendas e Manutenções de Equipamentos ZEBRA",
  telefone: "Tel: +55 71 98223-5050 / +55 71 98165-0801",
  contato: "E-mail: bmitag@bmitag.com.br | bmitag.com.br",
  cnpj: "CNPJ: 57.522.734/0001-58",
};

function Cell({
  label,
  value,
  span,
}: {
  label: string;
  value?: string | null;
  span?: boolean;
}) {
  return (
    <div className={`border border-gray-300 p-2 ${span ? "col-span-2" : ""}`}>
      <p className="text-[10px] font-semibold uppercase tracking-wide text-gray-500">
        {label}
      </p>
      <p className="mt-0.5 text-sm text-gray-900">{value?.trim() || "—"}</p>
    </div>
  );
}

export function DocumentoPreview({
  tipo,
  equipamento,
  verificacao,
}: {
  tipo: TipoDocumentoOficina;
  equipamento: EquipamentoAutoOS;
  verificacao: VerificacaoAutoOS | null;
}) {
  const os = numeroOS(equipamento.id);
  const titulo = tipo === "orcamento" ? "ORÇAMENTO TÉCNICO" : "ORDEM DE SERVIÇO";
  const empresa = equipamento.cliente_nome || "—";
  const responsavel =
    equipamento.responsavel_nome || equipamento.cliente_nome || "—";
  const contato = [
    equipamento.responsavel_telefone || equipamento.cliente_telefone,
    equipamento.responsavel_email || equipamento.cliente_email,
  ]
    .filter(Boolean)
    .join(" · ");
  const servicos = parseServicosOrcamento(verificacao?.servicos_necessarios);
  const pecas = parsePecasOrcamento(verificacao?.pecas_necessarias);
  const total = totalOrcamento({
    servicos,
    pecas,
    custoTotal: verificacao?.custo_total ?? equipamento.valor_orcamento,
  });
  const status = labelStatusEquipamento(equipamento.status);
  const defeito =
    equipamento.defeito_relatado || verificacao?.problema_relatado || "—";
  const diagnostico = verificacao?.diagnostico || "—";

  return (
    <article className="mx-auto max-w-[210mm] bg-white p-8 text-gray-900 shadow-lg">
      <header className="border-b border-gray-300 pb-4">
        <p className="text-sm font-bold tracking-wide">{CABECALHO.nome}</p>
        <p className="text-xs text-gray-600">{CABECALHO.descricao}</p>
        <p className="text-xs text-gray-600">{CABECALHO.telefone}</p>
        <p className="text-xs text-gray-600">{CABECALHO.contato}</p>
        <p className="text-xs text-gray-600">{CABECALHO.cnpj}</p>
        <div className="mt-4 flex items-end justify-between">
          <h2 className="text-lg font-bold">{titulo}</h2>
          <div className="text-right text-xs">
            <p className="font-mono font-semibold">{os}</p>
            <p>
              {tipo === "orcamento" ? "Emissão" : "Entrada"}:{" "}
              {formatDatePtBr(
                tipo === "orcamento" ? new Date().toISOString() : equipamento.data_entrada,
              )}
            </p>
          </div>
        </div>
      </header>

      {tipo === "ordem_servico" ? (
        <div className="mt-4 grid grid-cols-2">
          <Cell label="Status atual" value={status} span />
          <Cell
            label="Empresa cliente"
            value={`${empresa}${equipamento.cliente_documento ? ` · ${equipamento.cliente_documento}` : ""}`}
          />
          <Cell label="Contato responsável" value={responsavel} />
          <Cell label="Contato" value={contato} span />
          <Cell label="Equipamento" value={`${equipamento.marca} ${equipamento.modelo}`} />
          <Cell label="Tipo" value={equipamento.tipo} />
          <Cell label="Nº de série" value={equipamento.serial_number} />
          <Cell label="Patrimônio" value={equipamento.patrimonio} />
          <Cell label="Defeito informado" value={defeito} span />
          <Cell label="Laudo técnico" value={diagnostico} span />
          <Cell label="Acessórios" value={equipamento.acessorios} />
          <Cell label="Outros acessórios" value={equipamento.acessorios_outros} />
          <Cell label="Observações" value={equipamento.observacoes} span />
        </div>
      ) : (
        <div className="mt-4 space-y-4">
          <div className="grid grid-cols-3 text-sm">
            <Cell label="Empresa" value={empresa} />
            <Cell label="Responsável pelo equipamento" value={responsavel} />
            <Cell label="Contato" value={contato} />
          </div>
          {(servicos.length > 0 || pecas.length > 0 || total > 0) && (
            <div>
              <p className="mb-2 text-[10px] font-semibold uppercase tracking-wide text-gray-500">
                Planilha de valores
              </p>
              <table className="w-full text-left text-sm">
                <thead>
                  <tr className="border-b border-gray-300 text-xs uppercase text-gray-500">
                    <th className="py-1">Descrição</th>
                    <th>Modelo</th>
                    <th className="text-right">Qtd</th>
                    <th className="text-right">Valor</th>
                  </tr>
                </thead>
                <tbody>
                  {servicos.map((item, index) => (
                    <tr key={`s-${index}`} className="border-b border-gray-200">
                      <td className="py-1.5">{item.descricao}</td>
                      <td>
                        {equipamento.marca} {equipamento.modelo}
                      </td>
                      <td className="text-right">01</td>
                      <td className="text-right">{formatCurrency(item.valor)}</td>
                    </tr>
                  ))}
                  {pecas.map((item, index) => (
                    <tr key={`p-${index}`} className="border-b border-gray-200">
                      <td className="py-1.5">{item.nome}</td>
                      <td>
                        {equipamento.marca} {equipamento.modelo}
                      </td>
                      <td className="text-right">
                        {String(item.quantidade).padStart(2, "0")}
                      </td>
                      <td className="text-right">
                        {formatCurrency(item.valorTotal ?? item.valor_total ?? 0)}
                      </td>
                    </tr>
                  ))}
                  {servicos.length === 0 && pecas.length === 0 && total > 0 && (
                    <tr className="border-b border-gray-200">
                      <td className="py-1.5">Serviços técnicos</td>
                      <td>
                        {equipamento.marca} {equipamento.modelo}
                      </td>
                      <td className="text-right">01</td>
                      <td className="text-right">{formatCurrency(total)}</td>
                    </tr>
                  )}
                </tbody>
              </table>
              <p className="mt-3 text-right text-sm font-bold">
                VALOR TOTAL: {formatCurrency(total)}
              </p>
            </div>
          )}
          <div className="grid grid-cols-2">
            <Cell label="Nº de série" value={equipamento.serial_number} />
            <Cell
              label="Equipamento"
              value={`${equipamento.marca} ${equipamento.modelo}`}
            />
          </div>
          {verificacao?.diagnostico && (
            <Cell label="Diagnóstico" value={verificacao.diagnostico} span />
          )}
          {verificacao?.tecnico_nome && (
            <p className="text-xs text-gray-600">
              Técnico responsável: {verificacao.tecnico_nome}
            </p>
          )}
        </div>
      )}
    </article>
  );
}
