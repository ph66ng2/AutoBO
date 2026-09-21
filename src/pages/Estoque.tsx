import { FormEvent, useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, ArrowDownCircle, ArrowUpCircle, Package, Plus, RefreshCw, Search, X } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Badge } from "../components/ui/badge";
import { Label } from "../components/ui/label";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import {
  atualizarCadastroProdutoAutoos,
  criarProdutoAutoos,
  listarMovimentacoesProduto,
  listarProdutosAutoos,
  registrarMovimentacaoEstoque,
} from "../lib/db";
import { CATEGORIA_OPTIONS, ORIGEM_OPTIONS } from "../lib/estoque-catalogo";
import { formatCurrency } from "../lib/format";
import type { MovimentacaoEstoque, ProdutoAutoOS, ProdutoCadastroInput } from "../types";

const VAZIO: ProdutoCadastroInput = {
  codigo: "",
  nome: "",
  descricao: "",
  categoria: "ROLO",
  preco_custo: 0,
  preco_venda: 0,
  quantidade_inicial: 0,
  quantidade_minima: 5,
  unidade_medida: "UN",
  localizacao: "",
};

const selectClass =
  "h-10 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground";

function rotuloProduto(p: ProdutoAutoOS): string {
  return `${p.codigo} — ${p.nome}`;
}

/** Busca com autofill restrita às opções (mesmo comportamento do Select do AutoOS,
 *  mas com digitação para filtrar). Só aceita um produto da lista. */
function ProdutoCombobox({
  produtos,
  produtoId,
  onSelect,
}: {
  produtos: ProdutoAutoOS[];
  produtoId: number;
  onSelect: (id: number) => void;
}) {
  const selecionado = produtos.find((p) => p.id === produtoId) ?? null;
  const [texto, setTexto] = useState(selecionado ? rotuloProduto(selecionado) : "");
  const [aberto, setAberto] = useState(false);
  const [destaque, setDestaque] = useState(0);
  const caixaRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setTexto(selecionado ? rotuloProduto(selecionado) : "");
  }, [produtoId]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    function aoClicarFora(e: MouseEvent) {
      if (caixaRef.current && !caixaRef.current.contains(e.target as Node)) {
        setAberto(false);
      }
    }
    document.addEventListener("mousedown", aoClicarFora);
    return () => document.removeEventListener("mousedown", aoClicarFora);
  }, []);

  const termo = texto.trim().toLowerCase();
  const filtrados = useMemo(() => {
    if (!termo) return produtos;
    return produtos.filter((p) =>
      `${p.codigo} ${p.nome} ${p.descricao ?? ""}`.toLowerCase().includes(termo),
    );
  }, [produtos, termo]);

  useEffect(() => {
    setDestaque(0);
  }, [termo]);

  function escolher(p: ProdutoAutoOS) {
    onSelect(p.id);
    setTexto(rotuloProduto(p));
    setAberto(false);
  }

  function confirmarTexto() {
    if (!texto.trim()) {
      onSelect(0);
      setAberto(false);
      return;
    }
    const exato = produtos.find(
      (p) =>
        rotuloProduto(p).toLowerCase() === texto.trim().toLowerCase() ||
        p.codigo.toLowerCase() === texto.trim().toLowerCase() ||
        p.nome.toLowerCase() === texto.trim().toLowerCase(),
    );
    const alvo = exato ?? filtrados[destaque] ?? filtrados[0];
    if (alvo) {
      escolher(alvo);
    } else {
      setTexto(selecionado ? rotuloProduto(selecionado) : "");
      setAberto(false);
    }
  }

  return (
    <div ref={caixaRef} className="relative">
      <div className="relative">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          className="pl-9 pr-9"
          placeholder="Digite para buscar por código ou nome..."
          value={texto}
          onChange={(e) => {
            setTexto(e.target.value);
            setAberto(true);
          }}
          onFocus={() => setAberto(true)}
          onBlur={() => {
            setTimeout(() => {
              setAberto(false);
              confirmarTexto();
            }, 120);
          }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setAberto(true);
              setDestaque((d) => Math.min(d + 1, Math.max(filtrados.length - 1, 0)));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setDestaque((d) => Math.max(d - 1, 0));
            } else if (e.key === "Enter") {
              e.preventDefault();
              confirmarTexto();
            } else if (e.key === "Escape") {
              setAberto(false);
            }
          }}
        />
        {texto && (
          <button
            type="button"
            aria-label="Limpar produto"
            className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-muted-foreground hover:text-foreground"
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => {
              onSelect(0);
              setTexto("");
              setAberto(true);
            }}
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>
      {aberto && (
        <div className="absolute z-20 mt-1 max-h-60 w-full overflow-y-auto rounded-md border bg-card shadow-lg">
          {filtrados.length === 0 ? (
            <p className="px-3 py-2 text-sm text-muted-foreground">Nenhum produto encontrado</p>
          ) : (
            filtrados.slice(0, 50).map((p, i) => {
              const ativo = p.id === produtoId;
              const focado = i === destaque;
              return (
                <button
                  key={p.id}
                  type="button"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => escolher(p)}
                  onMouseEnter={() => setDestaque(i)}
                  className={`flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-sm ${
                    focado ? "bg-muted" : ""
                  } ${ativo ? "text-primary" : "text-foreground"} hover:bg-muted`}
                >
                  <span className="truncate">
                    <span className="font-mono text-xs text-muted-foreground">{p.codigo}</span>
                    <span className="mx-1 text-muted-foreground">—</span>
                    {p.nome}
                  </span>
                  <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                    saldo {p.quantidade_estoque ?? 0}
                  </span>
                </button>
              );
            })
          )}
        </div>
      )}
      {selecionado && (
        <p className="mt-1 text-xs text-muted-foreground">
          Selecionado: <span className="font-medium text-foreground">{rotuloProduto(selecionado)}</span>
          {" · "}saldo {selecionado.quantidade_estoque ?? 0} {selecionado.unidade_medida || "un."}
        </p>
      )}
    </div>
  );
}

export default function Estoque() {
  const [produtos, setProdutos] = useState<ProdutoAutoOS[]>([]);
  const [busca, setBusca] = useState("");
  const [categoriaFiltro, setCategoriaFiltro] = useState("TODOS");
  const [apenasEstoqueBaixo, setApenasEstoqueBaixo] = useState(false);
  const [carregando, setCarregando] = useState(true);
  const [form, setForm] = useState<ProdutoCadastroInput>(VAZIO);
  const [editandoId, setEditandoId] = useState<number | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [salvando, setSalvando] = useState(false);
  const [movimento, setMovimento] = useState({
    produtoId: 0,
    tipo: "ENTRADA" as "ENTRADA" | "SAIDA",
    quantidade: 1,
    origem: "COMPRA",
    referencia: "",
  });
  const [trilha, setTrilha] = useState<MovimentacaoEstoque[]>([]);
  const [erro, setErro] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);

  async function recarregar() {
    setCarregando(true);
    try {
      const lista = await listarProdutosAutoos({
        busca,
        categoria: categoriaFiltro,
        apenasEstoqueBaixo,
      });
      setProdutos(lista);
      setErro(null);
    } catch (e) {
      setErro(String(e));
    } finally {
      setCarregando(false);
    }
  }

  useEffect(() => {
    const t = setTimeout(() => {
      recarregar();
    }, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [busca, categoriaFiltro, apenasEstoqueBaixo]);

  const abaixoMinimo = useMemo(
    () =>
      produtos.filter(
        (p) => (p.quantidade_estoque ?? 0) < (p.quantidade_minima ?? 0),
      ).length,
    [produtos],
  );

  function abrirNovo() {
    setEditandoId(null);
    setForm(VAZIO);
    setDialogOpen(true);
  }

  function editar(p: ProdutoAutoOS) {
    setEditandoId(p.id);
    setForm({
      codigo: p.codigo,
      nome: p.nome,
      descricao: p.descricao ?? "",
      categoria: p.categoria,
      preco_custo: p.preco_custo ?? 0,
      preco_venda: p.preco_venda ?? 0,
      quantidade_minima: p.quantidade_minima,
      quantidade_maxima: p.quantidade_maxima,
      unidade_medida: p.unidade_medida ?? "UN",
      localizacao: p.localizacao ?? "",
    });
    setMovimento((m) => ({ ...m, produtoId: p.id }));
    setDialogOpen(true);
    listarMovimentacoesProduto(p.id).then(setTrilha).catch(() => setTrilha([]));
  }

  async function salvarCadastro(e: FormEvent) {
    e.preventDefault();
    setErro(null);
    setOk(null);
    setSalvando(true);
    try {
      if (editandoId) {
        await atualizarCadastroProdutoAutoos(editandoId, form);
        setOk("Cadastro e preço atualizados. Saldo não foi alterado.");
      } else {
        await criarProdutoAutoos(form);
        setOk("Produto cadastrado na mesma tabela do AutoOS.");
      }
      setForm(VAZIO);
      setEditandoId(null);
      setDialogOpen(false);
      await recarregar();
    } catch (err) {
      setErro(String(err));
    } finally {
      setSalvando(false);
    }
  }

  async function mover(e: FormEvent) {
    e.preventDefault();
    setErro(null);
    setOk(null);
    try {
      const atualizado = await registrarMovimentacaoEstoque({
        produto_id: movimento.produtoId,
        tipo: movimento.tipo,
        quantidade: Number(movimento.quantidade),
        origem: movimento.origem,
        referencia: movimento.referencia.trim() || undefined,
      });
      setTrilha(await listarMovimentacoesProduto(atualizado.id));
      setOk(
        `${movimento.tipo === "ENTRADA" ? "Entrada" : "Baixa"} registrada. Novo saldo: ${atualizado.quantidade_estoque ?? 0}`,
      );
      await recarregar();
    } catch (err) {
      setErro(String(err));
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Insumos/Peças & Estoque</h1>
          <p className="text-muted-foreground">
            Mesma tabela `produtos` do AutoOS. Cadastro e preço no formulário; quantidade só por entrada ou baixa.
          </p>
        </div>
        <div className="flex items-center gap-2">
          {abaixoMinimo > 0 && (
            <Badge variant="danger">
              <AlertTriangle className="mr-1 h-3.5 w-3.5" />
              {abaixoMinimo} abaixo do mínimo
            </Badge>
          )}
          <Button type="button" onClick={abrirNovo}>
            <Plus className="mr-2 h-4 w-4" />
            Novo insumo/peça
          </Button>
        </div>
      </div>

      {erro && (
        <p className="rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">{erro}</p>
      )}
      {ok && (
        <p className="rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-800">{ok}</p>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Entrada / baixa</CardTitle>
        </CardHeader>
        <CardContent>
          <form className="space-y-3" onSubmit={mover}>
            <ProdutoCombobox
              produtos={produtos}
              produtoId={movimento.produtoId}
              onSelect={(id) => {
                setMovimento((m) => ({ ...m, produtoId: id }));
                if (id) {
                  listarMovimentacoesProduto(id).then(setTrilha).catch(() => setTrilha([]));
                }
              }}
            />
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              <select
                className={selectClass}
                value={movimento.tipo}
                onChange={(e) => setMovimento({ ...movimento, tipo: e.target.value as "ENTRADA" | "SAIDA" })}
              >
                <option value="ENTRADA">Entrada</option>
                <option value="SAIDA">Saída</option>
              </select>
              <Input
                type="number"
                min={1}
                value={movimento.quantidade}
                onChange={(e) => setMovimento({ ...movimento, quantidade: Number(e.target.value) })}
              />
              <select
                className={selectClass}
                value={movimento.origem}
                onChange={(e) => setMovimento({ ...movimento, origem: e.target.value })}
              >
                {ORIGEM_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
              <Input
                placeholder="Referência"
                value={movimento.referencia}
                onChange={(e) => setMovimento({ ...movimento, referencia: e.target.value })}
              />
            </div>
            {(() => {
              const atual = produtos.find((p) => p.id === movimento.produtoId);
              const saldo = atual?.quantidade_estoque ?? 0;
              const qtd = Number(movimento.quantidade) || 0;
              const resultante = movimento.tipo === "ENTRADA" ? saldo + qtd : saldo - qtd;
              return atual ? (
                <p className="flex items-center gap-1 text-xs text-muted-foreground">
                  {movimento.tipo === "ENTRADA" ? (
                    <ArrowUpCircle className="h-3.5 w-3.5 text-emerald-700" />
                  ) : (
                    <ArrowDownCircle className="h-3.5 w-3.5 text-destructive" />
                  )}
                  Saldo atual: <span className="font-bold tabular-nums text-foreground">{saldo}</span>
                  {qtd > 0 && (
                    <>
                      {" → "}resultante:{" "}
                      <span className={`font-bold tabular-nums ${resultante < 0 ? "text-destructive" : "text-emerald-700"}`}>
                        {resultante}
                      </span>
                    </>
                  )}
                </p>
              ) : null;
            })()}
            <Button type="submit" disabled={!movimento.produtoId}>Registrar movimento</Button>
          </form>
          {trilha.length > 0 && (
            <ul className="mt-4 space-y-1 text-xs text-muted-foreground">
              {trilha.map((m) => (
                <li key={m.id}>
                  {m.tipo} {m.quantidade} · {m.origem} · {m.data_hora}
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-6">
          <div className="flex flex-col gap-3 sm:flex-row">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                className="pl-9"
                placeholder="Buscar por nome, código, marca..."
                value={busca}
                onChange={(e) => setBusca(e.target.value)}
              />
            </div>
            <select
              className={`${selectClass} sm:w-48`}
              value={categoriaFiltro}
              onChange={(e) => setCategoriaFiltro(e.target.value)}
            >
              {CATEGORIA_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
            <Button
              type="button"
              variant={apenasEstoqueBaixo ? "destructive" : "outline"}
              onClick={() => setApenasEstoqueBaixo((v) => !v)}
            >
              <AlertTriangle className="mr-1 h-4 w-4" />
              Estoque baixo
            </Button>
            <Button type="button" variant="outline" size="icon" onClick={() => recarregar()} aria-label="Atualizar">
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Produtos</CardTitle>
        </CardHeader>
        <CardContent className="overflow-x-auto">
          {carregando ? (
            <div className="flex h-32 items-center justify-center">
              <div className="h-8 w-8 animate-spin rounded-full border-b-2 border-primary" />
            </div>
          ) : produtos.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              <Package className="mx-auto mb-4 h-16 w-16 opacity-20" />
              <p className="text-lg font-medium">Nenhum insumo/peça encontrado</p>
              <p className="text-sm">
                {busca || categoriaFiltro !== "TODOS" || apenasEstoqueBaixo
                  ? "Tente ajustar os filtros"
                  : "Cadastre o primeiro item ou verifique a conexão com o AutoOS"}
              </p>
            </div>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-muted-foreground">
                  <th className="py-2">Nome</th>
                  <th>Código</th>
                  <th>Categoria</th>
                  <th>Descrição</th>
                  <th className="text-center">Estoque</th>
                  <th className="text-center">Mínimo</th>
                  <th className="text-right">Venda</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {produtos.map((p) => {
                  const saldo = p.quantidade_estoque ?? 0;
                  const minimo = p.quantidade_minima ?? 0;
                  const baixo = saldo < minimo;
                  return (
                    <tr key={p.id} className={`border-b ${baixo ? "bg-red-50" : ""}`}>
                      <td className="py-2">
                        <span className="font-medium">{p.nome}</span>
                        {baixo && <AlertTriangle className="ml-2 inline h-3.5 w-3.5 text-destructive" />}
                      </td>
                      <td className="font-mono text-xs">{p.codigo}</td>
                      <td>
                        <Badge variant="neutral">{p.categoria}</Badge>
                      </td>
                      <td className="max-w-xs truncate text-muted-foreground">{p.descricao || "—"}</td>
                      <td className="text-center">
                        <span className={`font-bold tabular-nums ${baixo ? "text-destructive" : "text-emerald-700"}`}>
                          {saldo}
                        </span>
                        <span className="ml-1 text-xs text-muted-foreground">{p.unidade_medida || "un."}</span>
                      </td>
                      <td className="text-center tabular-nums text-muted-foreground">{minimo}</td>
                      <td className="text-right tabular-nums">{formatCurrency(p.preco_venda ?? 0)}</td>
                      <td className="text-right">
                        <Button type="button" size="sm" variant="outline" onClick={() => editar(p)}>
                          Editar
                        </Button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{editandoId ? "Editar Insumo/Peça" : "Novo Insumo/Peça"}</DialogTitle>
          </DialogHeader>
          <form onSubmit={salvarCadastro} className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Nome *</Label>
                <Input
                  placeholder="Toner HP 26A"
                  value={form.nome}
                  onChange={(e) => setForm({ ...form, nome: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label>Código *</Label>
                <Input
                  placeholder="SKU-001"
                  value={form.codigo}
                  onChange={(e) => setForm({ ...form, codigo: e.target.value })}
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Categoria *</Label>
                <select
                  className={selectClass}
                  value={form.categoria}
                  onChange={(e) => setForm({ ...form, categoria: e.target.value })}
                >
                  {CATEGORIA_OPTIONS.filter((c) => c.value !== "TODOS").map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
              <div className="space-y-2">
                <Label>Unidade</Label>
                <Input
                  value={form.unidade_medida ?? "UN"}
                  onChange={(e) => setForm({ ...form, unidade_medida: e.target.value })}
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label>Descrição</Label>
              <Input
                placeholder="Descrição do produto..."
                value={form.descricao ?? ""}
                onChange={(e) => setForm({ ...form, descricao: e.target.value })}
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Preço custo</Label>
                <Input
                  type="number"
                  step="0.01"
                  min={0}
                  value={form.preco_custo}
                  onChange={(e) => setForm({ ...form, preco_custo: Number(e.target.value) })}
                />
              </div>
              <div className="space-y-2">
                <Label>Preço venda</Label>
                <Input
                  type="number"
                  step="0.01"
                  min={0}
                  value={form.preco_venda}
                  onChange={(e) => setForm({ ...form, preco_venda: Number(e.target.value) })}
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Qtd. mínima *</Label>
                <Input
                  type="number"
                  min={0}
                  value={form.quantidade_minima ?? 5}
                  onChange={(e) => setForm({ ...form, quantidade_minima: Number(e.target.value) })}
                />
              </div>
              {editandoId ? (
                <p className="self-end text-xs text-muted-foreground">
                  Saldo atual não é editado aqui. Use entrada/baixa.
                </p>
              ) : (
                <div className="space-y-2">
                  <Label>Saldo inicial</Label>
                  <Input
                    type="number"
                    min={0}
                    value={form.quantidade_inicial ?? 0}
                    onChange={(e) => setForm({ ...form, quantidade_inicial: Number(e.target.value) })}
                  />
                </div>
              )}
            </div>
            <div className="space-y-2">
              <Label>Localização</Label>
              <Input
                placeholder="Prateleira A, Gaveta 3..."
                value={form.localizacao ?? ""}
                onChange={(e) => setForm({ ...form, localizacao: e.target.value })}
              />
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  Cancelar
                </Button>
              </DialogClose>
              <Button type="submit" disabled={salvando}>
                {salvando ? "Salvando..." : editandoId ? "Salvar" : "Cadastrar"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
