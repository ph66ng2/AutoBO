import { invoke } from "@tauri-apps/api/core";

export type AuthRole = "admin" | "operator" | "fiscal";
export type AuthPermission = "operate" | "manage_company" | "fiscal";

export interface PublicSession {
  accountId: string;
  companyId: string;
  email: string;
  role: AuthRole;
  permissions: AuthPermission[];
  expiresAt: number;
}

export interface CompanyOption {
  companyId: string;
  label: string;
  role: AuthRole;
}

export type AuthView =
  | {
      kind: "signed_out" | "locked" | "expired" | "needs_company" | "authenticated";
      message?: string;
      email?: string;
      companies?: CompanyOption[];
      session?: PublicSession;
    };

export function hasPermission(session: PublicSession | undefined, permission: AuthPermission): boolean {
  return !!session?.permissions.includes(permission);
}

export function hasFiscal(session: PublicSession | undefined): boolean {
  return hasPermission(session, "fiscal");
}

export function loginAutobo(email: string, password: string): Promise<AuthView> {
  return invoke("login_autobo", { email, password });
}

export function restaurarSessaoAutobo(): Promise<AuthView> {
  return invoke("restaurar_sessao_autobo");
}

export function selecionarEmpresaAutobo(companyId: string): Promise<AuthView> {
  return invoke("selecionar_empresa_autobo", { companyId });
}

export function bloquearSessaoAutobo(): Promise<AuthView> {
  return invoke("bloquear_sessao_autobo");
}

export function sairSessaoAutobo(): Promise<AuthView> {
  return invoke("sair_sessao_autobo");
}
