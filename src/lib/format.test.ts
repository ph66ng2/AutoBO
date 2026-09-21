import { describe, expect, it } from "vitest";
import { formatCurrency, formatDatePtBr } from "./format";

describe("formatCurrency", () => {
  it("formats BRL", () => {
    expect(formatCurrency(1250)).toContain("1.250");
  });
});

describe("formatDatePtBr", () => {
  it("keeps YYYY-MM-DD in the local calendar", () => {
    expect(formatDatePtBr("2026-09-19")).toBe("19/09/2026");
  });
});
