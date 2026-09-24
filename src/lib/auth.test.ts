import { describe, expect, it } from "vitest";
import { hasFiscal, hasPermission, type PublicSession } from "./auth";

const admin: PublicSession = {
  accountId: "a0000000-0000-4000-8000-000000000001",
  companyId: "b0000000-0000-4000-8000-000000000010",
  email: "admin.a@example.test",
  role: "admin",
  permissions: ["operate", "manage_company"],
  expiresAt: 1_900_000_000,
};

const fiscal: PublicSession = {
  ...admin,
  role: "fiscal",
  permissions: ["fiscal"],
};

describe("permissões do AutoBO", () => {
  it("não trata admin AutoBO como fiscal", () => {
    expect(hasFiscal(admin)).toBe(false);
    expect(hasPermission(admin, "manage_company")).toBe(true);
  });

  it("isola a permissão fiscal no papel fiscal", () => {
    expect(hasFiscal(fiscal)).toBe(true);
    expect(hasPermission(fiscal, "manage_company")).toBe(false);
  });
});
