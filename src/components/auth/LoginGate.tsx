import { useEffect, useState } from "react";
import { AlertCircle, ArrowRight, Building2, Loader2, LockKeyhole } from "lucide-react";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { useAuth } from "../../hooks/useAuth";

export function LoginGate() {
  const { view, login, selectCompany, retry } = useAuth();
  const [email, setEmail] = useState(view.email ?? "");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (view.email) setEmail(view.email);
    setError(view.kind === "authenticated" ? null : view.message ?? null);
  }, [view.email, view.kind, view.message]);

  async function handleLogin() {
    if (!email.trim() || !password) return;
    setError(null);
    setSubmitting(true);
    try {
      await login(email, password);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Não foi possível entrar.");
    } finally {
      setSubmitting(false);
      setPassword("");
    }
  }

  if (view.kind === "needs_company") {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background px-4">
        <section className="w-full max-w-md animate-[fadeIn_200ms_cubic-bezier(0.2,0,0,1)] rounded-3xl border bg-card p-6 shadow">
          <div className="mb-5 space-y-1">
            <p className="text-sm text-muted-foreground">Conta {view.email}</p>
            <h1 className="text-2xl font-semibold tracking-tight">Escolha a empresa</h1>
            <p className="text-sm text-muted-foreground">Somente empresas com acesso ao AutoBO aparecem aqui.</p>
          </div>
          <div className="space-y-2">
            {(view.companies ?? []).map((company) => (
              <Button
                key={company.companyId}
                type="button"
                variant="outline"
                className="h-12 w-full justify-start rounded-xl"
                onClick={() => void selectCompany(company.companyId)}
              >
                <Building2 className="h-4 w-4" />
                <span className="flex flex-col items-start">
                  <span>{company.label}</span>
                  <span className="text-xs font-normal text-muted-foreground">{company.role}</span>
                </span>
              </Button>
            ))}
          </div>
        </section>
      </main>
    );
  }

  const locked = view.kind === "locked";
  const notice = error ?? view.message;

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4">
      <section className="w-full max-w-md rounded-3xl border bg-card p-6 shadow">
        <div className="mb-6 space-y-2" style={{ animation: "fadeIn 200ms cubic-bezier(0.2, 0, 0, 1)" }}>
          <div className="flex h-12 w-12 items-center justify-center rounded-2xl border bg-secondary">
            <LockKeyhole className="h-5 w-5 text-foreground" />
          </div>
          <h1 className="text-2xl font-semibold tracking-tight">{locked ? "Sessão bloqueada" : "Entrar no AutoBO"}</h1>
          <p className="text-sm text-muted-foreground">
            A mesma conta do AutoOS, com permissão própria do AutoBO.
          </p>
        </div>
        <form
          className="space-y-4"
          style={{ animation: "fadeIn 200ms cubic-bezier(0.2, 0, 0, 1) 100ms both" }}
          onSubmit={(event) => {
            event.preventDefault();
            void handleLogin();
          }}
        >
          <div className="space-y-1.5">
            <Label htmlFor="autobo-email">Email</Label>
            <Input
              id="autobo-email"
              type="email"
              autoComplete="username"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder="financeiro@empresa.com.br"
              autoFocus={!view.email}
              required
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="autobo-password">Senha</Label>
            <Input
              id="autobo-password"
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder="Sua senha"
              autoFocus={!!view.email}
              required
            />
          </div>
          {notice && (
            <div role="alert" className="flex items-start gap-2 rounded-xl border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive">
              <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
              <span>{notice}</span>
            </div>
          )}
          <Button type="submit" className="h-11 w-full" disabled={submitting || !email.trim() || !password}>
            {submitting ? <Loader2 className="h-4 w-4 animate-spin" /> : <ArrowRight className="h-4 w-4" />}
            {submitting ? "Entrando..." : locked ? "Desbloquear" : "Entrar"}
          </Button>
          {view.kind === "expired" && (
            <button type="button" className="w-full text-sm text-muted-foreground hover:text-foreground" onClick={() => void retry()}>
              Tentar restaurar de novo
            </button>
          )}
        </form>
      </section>
    </main>
  );
}
