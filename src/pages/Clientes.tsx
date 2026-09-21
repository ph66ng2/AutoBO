import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Input } from "../components/ui/input";
import { listarClientesAutoos } from "../lib/db";
import type { ClienteAutoOS } from "../types";

export default function Clientes() {
  const [busca, setBusca] = useState("");
  const [clientes, setClientes] = useState<ClienteAutoOS[]>([]);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    const t = setTimeout(() => {
      listarClientesAutoos(busca)
        .then(setClientes)
        .catch((e) => setErro(String(e)));
    }, 200);
    return () => clearTimeout(t);
  }, [busca]);

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Clientes</h1>
        <p className="text-muted-foreground">Somente leitura — cadastro da oficina no AutoOS.</p>
      </div>
      <Input
        placeholder="Buscar por nome, razão social ou documento"
        value={busca}
        onChange={(e) => setBusca(e.target.value)}
      />
      {erro && <p className="text-sm text-destructive">{erro}</p>}
      <Card>
        <CardHeader>
          <CardTitle>{clientes.length} clientes</CardTitle>
        </CardHeader>
        <CardContent>
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b text-left text-muted-foreground">
                <th className="py-2">Nome</th>
                <th>Documento</th>
                <th>Contato</th>
                <th>Cidade</th>
              </tr>
            </thead>
            <tbody>
              {clientes.map((c) => (
                <tr key={c.id} className="border-b">
                  <td className="py-2">{c.nome || c.razao_social || "—"}</td>
                  <td>{c.documento || c.cpf_cnpj || "—"}</td>
                  <td>
                    {c.telefone || "—"}
                    {c.email ? ` · ${c.email}` : ""}
                  </td>
                  <td>
                    {[c.cidade, c.uf].filter(Boolean).join("/") || "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </CardContent>
      </Card>
    </div>
  );
}
