import { NavLink, Outlet } from "react-router-dom";
import {
  LayoutDashboard,
  Receipt,
  FilePlus2,
  Users,
  Printer,
  Package,
  Settings,
} from "lucide-react";
import { cn } from "../../lib/cn";
import { useAuth } from "../../hooks/useAuth";
import type { IntegracaoAutoOS } from "../../types";

const linkClass = ({ isActive }: { isActive: boolean }) =>
  cn(
    "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium",
    isActive
      ? "bg-white/15 text-white shadow-sm"
      : "text-white/60 hover:bg-white/10 hover:text-white/90",
  );

export function AppLayout({ integracao }: { integracao: IntegracaoAutoOS }) {
  const { view, lock, signOut } = useAuth();
  const session = view.session;

  return (
    <div className="flex h-screen bg-background">
      <aside className="flex w-60 flex-col bg-sidebar">
        <div className="flex h-[150px] items-center justify-center border-b border-sidebar-border px-4">
          <img src="/logo-tag-trasparente.svg" alt="BMITAG" className="h-[110px] w-auto" />
        </div>
        <nav className="flex flex-1 flex-col gap-1 p-3">
          <NavLink to="/" end className={linkClass}>
            <LayoutDashboard className="h-5 w-5 shrink-0" />
            Dashboard
          </NavLink>
          <NavLink to="/boletos" className={linkClass}>
            <Receipt className="h-5 w-5 shrink-0" />
            Boletos
          </NavLink>
          <NavLink to="/novo-boleto" className={linkClass}>
            <FilePlus2 className="h-5 w-5 shrink-0" />
            Novo boleto
          </NavLink>
          {integracao.clientes && (
            <NavLink to="/clientes" className={linkClass}>
              <Users className="h-5 w-5 shrink-0" />
              Clientes
            </NavLink>
          )}
          {integracao.equipamentos && (
            <NavLink to="/equipamentos" className={linkClass}>
              <Printer className="h-5 w-5 shrink-0" />
              OS / Orçamentos
            </NavLink>
          )}
          {integracao.produtos && (
            <NavLink to="/estoque" className={linkClass}>
              <Package className="h-5 w-5 shrink-0" />
              Estoque
            </NavLink>
          )}
          <NavLink to="/configuracoes" className={linkClass}>
            <Settings className="h-5 w-5 shrink-0" />
            Sicredi
          </NavLink>
        </nav>
        <div className="space-y-2 border-t border-sidebar-border p-3">
          {session && (
            <div className="px-1">
              <p className="truncate text-xs text-white/80">{session.email}</p>
              <p className="text-[11px] uppercase tracking-wide text-white/40">{session.role}</p>
            </div>
          )}
          <div className="flex gap-1">
            <button
              type="button"
              className="flex-1 rounded-md px-2 py-1.5 text-xs text-white/60 transition-[background-color,color] duration-150 hover:bg-white/10 hover:text-white"
              onClick={() => void lock()}
            >
              Bloquear
            </button>
            <button
              type="button"
              className="flex-1 rounded-md px-2 py-1.5 text-xs text-white/60 transition-[background-color,color] duration-150 hover:bg-white/10 hover:text-white"
              onClick={() => void signOut()}
            >
              Sair
            </button>
          </div>
        </div>
      </aside>
      <main className="flex-1 overflow-auto bg-background">
        <div className="mx-auto max-w-7xl p-6">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
