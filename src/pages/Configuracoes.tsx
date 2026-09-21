import { useCallback, useEffect, useState } from "react";
import { Button } from "../components/ui/button";
import { Card, CardContent } from "../components/ui/card";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import {
  listarConfigSicredi,
  probeKeyring,
  setConfig,
  sincronizarSicrediAgora,
  testarSicredi,
} from "../lib/db";

type SicrediForm = {
  ambiente: string;
  cooperativa: string;
  posto: string;
  codigo_beneficiario: string;
  especie_documento: string;
  data_entrada_producao: string;
  api_key: string;
  codigo_acesso: string;
};

const VAZIO: SicrediForm = {
  ambiente: "sandbox",
  cooperativa: "",
  posto: "",
  codigo_beneficiario: "",
  especie_documento: "DUPLICATA_MERCANTIL_INDICACAO",
  data_entrada_producao: "",
  api_key: "",
  codigo_acesso: "",
};

export default function Configuracoes() {
  const [form, setForm] = useState<SicrediForm>(VAZIO);
  const [apiKeyOk, setApiKeyOk] = useState(false);
  const [acessoOk, setAcessoOk] = useState(false);
  const [keyringOk, setKeyringOk] = useState<boolean | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const carregar = useCallback(async () => {
    setErro(null);
    try {
      const data = await listarConfigSicredi();
      const v = data.valores as Record<string, unknown>;
      setForm({
        ambiente: String(v["sicredi.ambiente"] ?? "sandbox"),
        cooperativa: String(v["sicredi.cooperativa"] ?? ""),
        posto: String(v["sicredi.posto"] ?? ""),
        codigo_beneficiario: String(v["sicredi.codigo_beneficiario"] ?? ""),
        especie_documento: String(
          v["sicredi.especie_documento"] ?? "DUPLICATA_MERCANTIL_INDICACAO",
        ),
        data_entrada_producao: String(v["sicredi.data_entrada_producao"] ?? ""),
        api_key: "",
        codigo_acesso: "",
      });
      const ak = v["sicredi.api_key"] as { configurado?: boolean } | undefined;
      const ca = v["sicredi.codigo_acesso"] as { configurado?: boolean } | undefined;
      setApiKeyOk(!!ak?.configurado);
      setAcessoOk(!!ca?.configurado);
      setKeyringOk(!!data.keyring_disponivel);
    } catch (e) {
      setErro(String(e));
    }
  }, []);

  useEffect(() => {
    void carregar();
  }, [carregar]);

  async function salvar() {
    setBusy(true);
    setMsg(null);
    setErro(null);
    try {
      await setConfig("sicredi.ambiente", form.ambiente);
      await setConfig("sicredi.cooperativa", form.cooperativa);
      await setConfig("sicredi.posto", form.posto);
      await setConfig("sicredi.codigo_beneficiario", form.codigo_beneficiario);
      await setConfig("sicredi.especie_documento", form.especie_documento);
      await setConfig("sicredi.data_entrada_producao", form.data_entrada_producao);
      if (form.api_key.trim()) {
        await setConfig("sicredi.api_key", form.api_key.trim());
      }
      if (form.codigo_acesso.trim()) {
        await setConfig("sicredi.codigo_acesso", form.codigo_acesso.trim());
      }
      setMsg("Configuração salva.");
      setForm((f) => ({ ...f, api_key: "", codigo_acesso: "" }));
      await carregar();
    } catch (e) {
      setErro(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function testar() {
    setBusy(true);
    setMsg(null);
    setErro(null);
    try {
      const ok = await testarSicredi();
      setMsg(ok ? "Sicredi OK (OAuth + liquidados/dia)." : "Falha no teste.");
    } catch (e) {
      setErro(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function sync() {
    setBusy(true);
    setMsg(null);
    setErro(null);
    try {
      await sincronizarSicrediAgora();
      setMsg("Sincronização de liquidações concluída.");
    } catch (e) {
      setErro(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function probe() {
    try {
      await probeKeyring();
      setKeyringOk(true);
      setMsg("Cofre do sistema disponível.");
    } catch (e) {
      setKeyringOk(false);
      setErro(String(e));
    }
  }

  return (
    <div className="mx-auto max-w-2xl space-y-4">
      <h1 className="text-3xl font-bold tracking-tight">Configuração Sicredi</h1>
      <Card>
        <CardContent className="space-y-4 p-6">
          {keyringOk === false && (
            <p className="rounded border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
              Cofre do sistema indisponível. Credenciais sensíveis não podem ser salvas.
            </p>
          )}
          {keyringOk === true && (
            <p className="text-xs text-muted-foreground">Cofre do sistema: disponível</p>
          )}

          <div className="grid grid-cols-2 gap-3">
            <div>
              <Label>Ambiente</Label>
              <select
                className="mt-1 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground"
                value={form.ambiente}
                onChange={(e) => setForm({ ...form, ambiente: e.target.value })}
              >
                <option value="sandbox">sandbox</option>
                <option value="producao">producao</option>
              </select>
            </div>
            <div>
              <Label>Cooperativa (4)</Label>
              <Input
                value={form.cooperativa}
                onChange={(e) => setForm({ ...form, cooperativa: e.target.value })}
              />
            </div>
            <div>
              <Label>Posto (2)</Label>
              <Input
                value={form.posto}
                onChange={(e) => setForm({ ...form, posto: e.target.value })}
              />
            </div>
            <div>
              <Label>Código beneficiário (5)</Label>
              <Input
                value={form.codigo_beneficiario}
                onChange={(e) =>
                  setForm({ ...form, codigo_beneficiario: e.target.value })
                }
              />
            </div>
          </div>

          <div>
            <Label>Espécie documental Sicredi</Label>
            <Input
              value={form.especie_documento}
              onChange={(e) =>
                setForm({ ...form, especie_documento: e.target.value })
              }
            />
          </div>

          <div>
            <Label>Data entrada produção (YYYY-MM-DD)</Label>
            <Input
              value={form.data_entrada_producao}
              onChange={(e) =>
                setForm({ ...form, data_entrada_producao: e.target.value })
              }
              placeholder="Para inicializar o cursor de liquidações"
            />
          </div>

          <div>
            <Label>
              API Key (x-api-key){apiKeyOk ? " — configurada" : ""}
            </Label>
            <Input
              type="password"
              autoComplete="off"
              placeholder={apiKeyOk ? "•••••••• (deixe vazio para manter)" : ""}
              value={form.api_key}
              onChange={(e) => setForm({ ...form, api_key: e.target.value })}
            />
          </div>

          <div>
            <Label>
              Código de acesso (password)
              {acessoOk ? " — configurado" : ""}
            </Label>
            <Input
              type="password"
              autoComplete="off"
              placeholder={acessoOk ? "•••••••• (deixe vazio para manter)" : ""}
              value={form.codigo_acesso}
              onChange={(e) =>
                setForm({ ...form, codigo_acesso: e.target.value })
              }
            />
            <p className="mt-1 text-xs text-muted-foreground">
              Em sandbox o password fixo do manual é usado automaticamente.
            </p>
          </div>

          {msg && <p className="text-sm text-emerald-700">{msg}</p>}
          {erro && <p className="text-sm text-destructive">{erro}</p>}

          <div className="flex flex-wrap gap-2">
            <Button disabled={busy} onClick={() => void salvar()}>
              Salvar
            </Button>
            <Button disabled={busy} variant="outline" onClick={() => void testar()}>
              Testar Sicredi
            </Button>
            <Button disabled={busy} variant="outline" onClick={() => void sync()}>
              Sincronizar liquidações
            </Button>
            <Button disabled={busy} variant="outline" onClick={() => void probe()}>
              Probe keyring
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
