import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  bloquearSessaoAutobo,
  loginAutobo,
  restaurarSessaoAutobo,
  sairSessaoAutobo,
  selecionarEmpresaAutobo,
  type AuthView,
} from "../lib/auth";

interface AuthContextValue {
  view: AuthView;
  login(email: string, password: string): Promise<void>;
  selectCompany(companyId: string): Promise<void>;
  retry(): Promise<void>;
  lock(): Promise<void>;
  signOut(): Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

function fallback(message: string): AuthView {
  return { kind: "signed_out", message };
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<AuthView>({ kind: "signed_out" });
  const booted = useRef(false);

  const restore = useCallback(async () => {
    try {
      setView(await restaurarSessaoAutobo());
    } catch (error) {
      setView(fallback(error instanceof Error ? error.message : "Não foi possível restaurar a sessão."));
    }
  }, []);

  useEffect(() => {
    if (booted.current) return;
    booted.current = true;
    void restore();
  }, [restore]);

  const value = useMemo<AuthContextValue>(
    () => ({
      view,
      async login(email, password) {
        try {
          setView(await loginAutobo(email, password));
        } catch (error) {
          const message = typeof error === "string" ? error : error instanceof Error ? error.message : "Não foi possível entrar.";
          setView({ kind: "signed_out", message, email });
          throw new Error(message);
        }
      },
      async selectCompany(companyId) {
        setView(await selecionarEmpresaAutobo(companyId));
      },
      retry: restore,
      async lock() {
        setView(await bloquearSessaoAutobo());
      },
      async signOut() {
        setView(await sairSessaoAutobo());
      },
    }),
    [restore, view],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) throw new Error("useAuth deve ser usado dentro de AuthProvider");
  return context;
}
