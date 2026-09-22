import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { NFeUploader } from "../components/nfe/NFeUploader";
import { NFeListaImportacao } from "../components/nfe/NFeListaImportacao";
import { BoletoRevisaoForm } from "../components/boleto/BoletoRevisaoForm";
import { Card, CardContent } from "../components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import type { NFeImportadaDTO } from "../types";

type Modo = "selecao" | "nfe-upload" | "manual" | "revisao" | "importacao-lote";

export default function NovoBoleto() {
  const navigate = useNavigate();
  const [modo, setModo] = useState<Modo>("selecao");
  const [importadas, setImportadas] = useState<NFeImportadaDTO[]>([]);

  function handleImportada(dto: NFeImportadaDTO) {
    if (dto.sucesso && dto.dados) {
      setImportadas([dto]);
      setModo("revisao");
    }
  }

  function handleGerarSelecionados(selecionados: NFeImportadaDTO[]) {
    if (selecionados.length > 0 && selecionados[0].dados) {
      setImportadas(selecionados);
      setModo("revisao");
    }
  }

  function fechar() {
    setModo("selecao");
    setImportadas([]);
  }

  const titulo =
    modo === "manual"
      ? "Novo boleto"
      : modo === "nfe-upload"
        ? "Importar NF-e"
        : modo === "revisao"
          ? "Revisar boleto"
          : "Notas importadas";

  return (
    <div className="mx-auto max-w-4xl">
      <h1 className="mb-6 text-3xl font-bold tracking-tight">Novo Boleto</h1>

      {modo === "selecao" && (
        <div className="grid grid-cols-2 gap-4">
          <Card
            className="cursor-pointer transition-colors hover:border-primary"
            onClick={() => setModo("nfe-upload")}
          >
            <CardContent className="!p-8 text-center">
              <h2 className="mb-2 text-lg font-semibold">Importar NF-e</h2>
              <p className="text-sm text-muted-foreground">
                Upload do XML. O boleto nasce da nota, não da OS.
              </p>
            </CardContent>
          </Card>
          <Card
            className="cursor-pointer transition-colors hover:border-primary"
            onClick={() => setModo("manual")}
          >
            <CardContent className="!p-8 text-center">
              <h2 className="mb-2 text-lg font-semibold">Entrada Manual</h2>
              <p className="text-sm text-muted-foreground">
                Sem XML: informe pagador, valor e vencimento.
              </p>
            </CardContent>
          </Card>
        </div>
      )}

      <Dialog open={modo !== "selecao"} onOpenChange={(open) => { if (!open) fechar(); }}>
        <DialogContent className="max-w-2xl max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{titulo}</DialogTitle>
          </DialogHeader>
          {modo === "nfe-upload" && (
            <NFeUploader onImportada={handleImportada} onVoltar={fechar} />
          )}
          {modo === "importacao-lote" && importadas.length > 0 && (
            <NFeListaImportacao
              resultados={importadas}
              onGerarSelecionados={handleGerarSelecionados}
              onVoltar={fechar}
            />
          )}
          {modo === "manual" && (
            <BoletoRevisaoForm
              modo="manual"
              onVoltar={fechar}
              onSucesso={() => navigate("/boletos")}
            />
          )}
          {modo === "revisao" && importadas[0]?.dados && (
            <BoletoRevisaoForm
              modo="nfe"
              dadosNFe={importadas[0].dados}
              avisos={importadas[0].avisos}
              onVoltar={fechar}
              onSucesso={() => navigate("/boletos")}
            />
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
