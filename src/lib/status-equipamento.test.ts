import { describe, expect, it } from "vitest";
import {
  corStatusEquipamento,
  deveMostrarOrcamento,
  labelStatusEquipamento,
  numeroOS,
} from "./status-equipamento";

describe("status de equipamento", () => {
  it("mostra o rótulo curto do AutoOS, não o código do banco", () => {
    expect(labelStatusEquipamento("AGUARDANDO_APROVACAO")).toBe("Aguard. Aprovação");
    expect(labelStatusEquipamento("ORCAMENTO_VENCIDO")).toBe("Orçam. Vencido");
    expect(labelStatusEquipamento("PRONTO")).toBe("Pronto");
  });

  it("usa a paleta do AutoOS", () => {
    expect(corStatusEquipamento("APROVADO")).toBe("bg-green-100 text-green-800");
    expect(corStatusEquipamento("AGUARDANDO_APROVACAO")).toBe(
      "bg-orange-100 text-orange-800",
    );
  });

  it("mostra orçamento quando o AutoOS mostraria o documento", () => {
    expect(deveMostrarOrcamento({ status: "RECEBIDO", valor_orcamento: 0 })).toBe(false);
    expect(deveMostrarOrcamento({ status: "AGUARDANDO_APROVACAO", valor_orcamento: 0 })).toBe(
      true,
    );
    expect(deveMostrarOrcamento({ status: "RECEBIDO", valor_orcamento: 350 })).toBe(true);
    expect(deveMostrarOrcamento({ status: "RECEBIDO", tem_verificacao: true })).toBe(true);
  });

  it("gera o número da OS no formato do AutoOS", () => {
    expect(numeroOS(41)).toBe("OS-00041");
  });
});
