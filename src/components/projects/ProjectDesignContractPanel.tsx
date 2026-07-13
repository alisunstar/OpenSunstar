import { useCallback, useEffect, useState, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { Copy, Download, Eye, FileText, Loader2, Paintbrush, Palette, Upload } from "lucide-react";
import { toast } from "sonner";
import { open } from "@tauri-apps/plugin-dialog";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import {
  designContractApi,
  type DesignContract,
  type DesignContractParams,
  type DesignColors,
  type DesignInstallResult,
  type DesignInstallPlan,
  type DesignSystemVerification,
  type DesignSystemPackage,
  type DesignSystemPackageDetail,
} from "@/lib/api/designContract";
import { InstallConfirmModal } from "@/components/shared/InstallConfirmModal";

const COLOR_FIELDS: { key: keyof DesignColors; label: string }[] = [
  { key: "primary", label: "Primary" }, { key: "primaryHover", label: "Primary Hover" },
  { key: "background", label: "Background" }, { key: "surface", label: "Surface" },
  { key: "textPrimary", label: "Text Primary" }, { key: "textMuted", label: "Text Muted" },
  { key: "accent", label: "Accent" }, { key: "success", label: "Success" },
  { key: "warning", label: "Warning" }, { key: "error", label: "Error" },
  { key: "border", label: "Border" },
];

/** Apply a loaded contract to all local state setters. */
function applyContract(
  c: DesignContract,
  set: {
    setContract: (v: DesignContract) => void;
    setColors: (v: DesignColors) => void;
    setFontBase: (v: string) => void;
    setFontHeading: (v: string) => void;
    setFontMono: (v: string) => void;
    setSpacingBase: (v: number) => void;
    setContractName: (v: string) => void;
    setActiveTab: (v: string) => void;
  },
) {
  set.setContract(c);
  set.setColors(c.colors);
  set.setFontBase(c.typography.fontFamilyBase);
  set.setFontHeading(c.typography.fontFamilyHeading);
  set.setFontMono(c.typography.fontFamilyMono);
  set.setSpacingBase(c.spacing.baseUnit);
  set.setContractName(c.name);
  set.setActiveTab("tokens");
}

function PrototypePreview({ page, components = [], index, colors, fontFamily, spacing }: { page: string | null; components?: string[]; index: number; colors?: DesignColors | null; fontFamily?: string; spacing?: number }) {
  const has = (name: string) => components.some((component) => component.toLowerCase().includes(name.toLowerCase()));
  const showTable = has("table") || has("list") || index % 3 === 0;
  const showForm = has("form") || has("editor") || index % 3 === 2;
  const showSidebar = has("sidebar") || has("split") || has("drawer") || index % 3 === 1;
  const showMetrics = has("metric") || has("chart") || has("dashboard");
  const tokenStyle = {
    "--proto-primary": colors?.primary ?? "hsl(var(--primary))",
    "--proto-surface": colors?.surface ?? "hsl(var(--card))",
    "--proto-background": colors?.background ?? "hsl(var(--background))",
    "--proto-text": colors?.textPrimary ?? "hsl(var(--foreground))",
    "--proto-muted": colors?.textMuted ?? "hsl(var(--muted-foreground))",
    "--proto-border": colors?.border ?? "hsl(var(--border))",
    "--proto-spacing": `${Math.max(spacing ?? 4, 1)}px`,
    fontFamily: fontFamily || undefined,
  } as CSSProperties;
  return <div style={tokenStyle} className="rounded-lg border border-[var(--proto-border)] bg-[var(--proto-surface)] p-[var(--proto-spacing)] text-[var(--proto-text)] shadow-inner" aria-label={`${page ?? "页面"} 静态原型`}>
    <div className="mb-3 flex items-center justify-between border-b border-[var(--proto-border)] pb-2"><div className="flex items-center gap-2"><div className="h-2.5 w-2.5 rounded-full bg-[var(--proto-primary)]" /><span className="text-[11px] font-semibold">{page ?? "页面模板"}</span></div><div className="flex gap-1"><span className="h-2 w-8 rounded bg-[var(--proto-muted)]/25" /><span className="h-2 w-5 rounded bg-[var(--proto-muted)]/15" /></div></div>
    {showMetrics && <div className="mb-3 grid grid-cols-3 gap-2">{["活跃用户", "转化率", "待处理"].map((label, i) => <div key={label} className="rounded-md border border-border/60 bg-background p-2"><div className="text-[9px] text-muted-foreground">{label}</div><div className="mt-1 text-sm font-semibold">{["12,480", "18.6%", "24"][i]}</div><div className="mt-2 h-1 rounded bg-primary/30"><div className="h-1 w-2/3 rounded bg-primary" /></div></div>)}</div>}
      <div className={cn("grid gap-[var(--proto-spacing)]", showSidebar ? "grid-cols-[150px_1fr]" : "grid-cols-1")}>
        {showSidebar && <aside className="rounded-md border border-[var(--proto-border)] bg-[var(--proto-background)] p-2"><div className="mb-2 h-2 w-16 rounded bg-[var(--proto-muted)]/30" /><div className="space-y-1.5">{["总览", "项目列表", "成员与权限", "设置"].map((item, i) => <div key={item} className={cn("rounded px-2 py-1.5 text-[9px]", i === 0 ? "bg-[var(--proto-primary)]/15 text-[var(--proto-primary)]" : "text-[var(--proto-muted)]")}>{item}</div>)}</div></aside>}
      <div className="space-y-3">
        {showTable && <div className="overflow-hidden rounded-md border border-border/60 bg-background"><div className="flex items-center justify-between border-b border-border/60 p-2"><div className="h-2 w-20 rounded bg-foreground/20" /><div className="h-6 w-16 rounded bg-primary/85" /></div><div className="grid grid-cols-3 gap-2 border-b border-border/50 px-2 py-1.5 text-[8px] text-muted-foreground"><span>名称</span><span>状态</span><span>更新时间</span></div>{["示例项目 A", "示例项目 B", "示例项目 C"].map((row) => <div key={row} className="grid grid-cols-3 gap-2 border-b border-border/40 px-2 py-2 text-[9px] last:border-0"><span>{row}</span><span className="text-emerald-500">进行中</span><span className="text-muted-foreground">今天</span></div>)}</div>}
        {showForm && <div className="rounded-md border border-border/60 bg-background p-3"><div className="mb-3 h-2 w-24 rounded bg-foreground/20" /><div className="grid grid-cols-2 gap-2"><div className="space-y-1"><span className="text-[8px] text-muted-foreground">名称</span><div className="h-7 rounded border border-border/70" /></div><div className="space-y-1"><span className="text-[8px] text-muted-foreground">状态</span><div className="h-7 rounded border border-border/70" /></div></div><div className="mt-2 space-y-1"><span className="text-[8px] text-muted-foreground">说明</span><div className="h-12 rounded border border-border/70" /></div><div className="mt-3 flex justify-end"><div className="h-7 w-16 rounded bg-primary/85" /></div></div>}
        {!showTable && !showForm && <div className="rounded-md border border-border/60 bg-background p-4"><div className="h-3 w-32 rounded bg-foreground/20" /><div className="mt-3 h-2 w-3/4 rounded bg-muted-foreground/20" /><div className="mt-2 h-2 w-1/2 rounded bg-muted-foreground/15" /><div className="mt-4 h-8 w-24 rounded bg-primary/85" /></div>}
      </div>
    </div>
  </div>;
}

// ──────────────────────── Main Component ────────────────────────

export default function ProjectDesignContractPanel({ projectId }: { projectId: string }) {
  const { t } = useTranslation();

  const [activeTab, setActiveTab] = useState("templates");
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [contract, setContract] = useState<DesignContract | null>(null);
  const [contractName, setContractName] = useState("My Design System");
  const [contractDesc, setContractDesc] = useState("");
  const [loading, setLoading] = useState(false);
  const [colors, setColors] = useState<DesignColors | null>(null);
  const [fontBase, setFontBase] = useState("Inter, system-ui, sans-serif");
  const [fontHeading, setFontHeading] = useState("Inter, system-ui, sans-serif");
  const [fontMono, setFontMono] = useState("JetBrains Mono, monospace");
  const [spacingBase, setSpacingBase] = useState(4);
  const [previewMd, setPreviewMd] = useState("");
  const [previewing, setPreviewing] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installResult, setInstallResult] = useState<DesignInstallResult | null>(null);
  const [importUrl, setImportUrl] = useState("");
  const [remoteImportKind, setRemoteImportKind] = useState<"github" | "shadcn">("github");
  const [importing, setImporting] = useState(false);
  const [installPlan, setInstallPlan] = useState<DesignInstallPlan | null>(null);
  const [exportPlan, setExportPlan] = useState<DesignInstallPlan | null>(null);
  const [packages, setPackages] = useState<DesignSystemPackage[]>([]);
  const [packageDetail, setPackageDetail] = useState<DesignSystemPackageDetail | null>(null);
  const [selectedPrototype, setSelectedPrototype] = useState<string | null>(null);
  const [verification, setVerification] = useState<DesignSystemVerification | null>(null);
  const [verifying, setVerifying] = useState(false);

  const setters = { setContract, setColors, setFontBase, setFontHeading, setFontMono, setSpacingBase, setContractName, setActiveTab };

  useEffect(() => {
    designContractApi.listPackages().then((result) => setPackages(result.packages)).catch(() => setPackages([]));
  }, []);

  const buildParams = useCallback((): DesignContractParams => ({
    templateId: selectedTemplate,
    prototypeTemplate: selectedPrototype,
    name: contractName || "My Design System",
    description: contractDesc || null,
    colors,
    typography: contract?.typography
      ? { ...contract.typography, fontFamilyBase: fontBase, fontFamilyHeading: fontHeading, fontFamilyMono: fontMono }
      : null,
    spacing: contract?.spacing ? { ...contract.spacing, baseUnit: spacingBase } : null,
    elevation: contract?.elevation ?? null,
    shapes: contract?.shapes ?? null,
    components: contract?.components ?? null,
    guardrails: contract?.guardrails ?? null,
  }), [selectedTemplate, selectedPrototype, contractName, contractDesc, colors, contract, fontBase, fontHeading, fontMono, spacingBase]);

  const loadTemplate = useCallback(async (id: string) => {
    setLoading(true);
    try {
      applyContract(await designContractApi.getPackageContract(id), setters);
      const detail = await designContractApi.getPackageDetail(id);
      setPackageDetail(detail);
      setSelectedPrototype(detail.components.pageTemplates?.[0] ?? null);
      setSelectedTemplate(id);
      // A package selection is a prototype task decision. Keep the user on
      // the package/flow view instead of dropping them into the legacy token
      // editor, which made the new packages look identical to the old presets.
      setActiveTab("templates");
    } catch (e) { toast.error(String(e)); }
    finally { setLoading(false); }
  }, []);

  const handlePreview = useCallback(async () => {
    setPreviewing(true);
    try { setPreviewMd(await designContractApi.previewDesignMd(buildParams())); setActiveTab("preview"); }
    catch (e) { toast.error(String(e)); }
    finally { setPreviewing(false); }
  }, [buildParams]);

  const refreshVerification = useCallback(async () => {
    setVerifying(true);
    try { setVerification(await designContractApi.verifyProjectSystem(projectId)); }
    catch (e) { toast.error(String(e)); }
    finally { setVerifying(false); }
  }, [projectId]);

  const handleCopy = useCallback(async () => {
    if (!previewMd) { await handlePreview(); return; }
    try { await navigator.clipboard.writeText(previewMd); toast.success(t("designContract.copied", { defaultValue: "Copied to clipboard" })); }
    catch { toast.error("Failed to copy"); }
  }, [previewMd, handlePreview, t]);

  const handleExportPreview = useCallback(async () => {
    setExporting(true);
    try {
      setExportPlan(await designContractApi.previewExportPlan(projectId, buildParams()));
    }
    catch (e) { toast.error(String(e)); }
    finally { setExporting(false); }
  }, [projectId, buildParams, t]);

  const handleExportConfirm = useCallback(async () => {
    setExportPlan(null);
    setExporting(true);
    try {
      const p = await designContractApi.exportContract(projectId, buildParams());
      toast.success(t("designContract.exported", { defaultValue: "已覆盖导出设计规范与 Tokens" }));
      setPreviewMd(p);
      setVerification(await designContractApi.verifyProjectSystem(projectId));
    }
    catch (e) { toast.error(String(e)); }
    finally { setExporting(false); }
  }, [projectId, buildParams, t]);

  const handleInstallPreview = useCallback(async () => {
    setInstalling(true);
    try {
      const plan = await designContractApi.previewInstallPlan(projectId, buildParams());
      setInstallPlan(plan);
    } catch (e) { toast.error(String(e)); }
    finally { setInstalling(false); }
  }, [projectId, buildParams]);

  const handleInstallConfirm = useCallback(async () => {
    setInstallPlan(null);
    setInstalling(true);
    try {
      setInstallResult(await designContractApi.installContract(projectId, buildParams()));
      setVerification(await designContractApi.verifyProjectSystem(projectId));
      toast.success(t("designContract.installed", { defaultValue: "Installed" }));
    } catch (e) { toast.error(String(e)); }
    finally { setInstalling(false); }
  }, [projectId, buildParams, t]);

  const handleImportFile = useCallback(async () => {
    const fp = await open({ filters: [{ name: "Markdown", extensions: ["md"] }] });
    if (!fp) return;
    setImporting(true);
    try {
      const r = await designContractApi.importFromFile(fp);
      applyContract(r.contract, setters);
      setSelectedTemplate(null);
      if (r.warnings.length) toast.warning(r.warnings.join(", "));
      if (r.quality.level === "needs_review") toast.warning(`导入内容需复核：缺少 ${r.quality.missingSections.join("、")}`);
    } catch (e) { toast.error(String(e)); }
    finally { setImporting(false); }
  }, []);

  const handleImportUrl = useCallback(async () => {
    if (!importUrl.trim()) return;
    setImporting(true);
    try {
      const r = await designContractApi.importFromUrl(
        await (await fetch(importUrl)).text(),
        importUrl,
        remoteImportKind,
      );
      applyContract(r.contract, setters);
      setSelectedTemplate(null);
      setImportUrl("");
      if (r.warnings.length) toast.warning(r.warnings.join(", "));
      if (r.quality.level === "needs_review") toast.warning(`导入内容需复核：缺少 ${r.quality.missingSections.join("、")}`);
    } catch (e) { toast.error(String(e)); }
    finally { setImporting(false); }
  }, [importUrl, remoteImportKind]);

  const updateColor = useCallback((key: keyof DesignColors, value: string) => {
    setColors((prev) => prev ? { ...prev, [key]: value } : prev);
  }, []);

  const LoadIcon = <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />;
  const prototypeIndex = packageDetail?.components.pageTemplates?.indexOf(selectedPrototype ?? "") ?? 0;

  return (
    <div className="space-y-4 p-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Palette className="h-4 w-4 text-primary" />
          <h3 className="text-sm font-semibold">{t("designContract.title", { defaultValue: "Design Contract" })}</h3>
        </div>
        {selectedTemplate && <Badge variant="secondary" className="text-[10px]">{selectedTemplate}</Badge>}
      </div>

      {/* Name & Description */}
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <Label className="text-[10px] font-medium text-muted-foreground uppercase">{t("designContract.name", { defaultValue: "Contract Name" })}</Label>
          <Input className="h-8 text-xs" value={contractName} onChange={(e) => setContractName(e.target.value)} />
        </div>
        <div className="space-y-1">
          <Label className="text-[10px] font-medium text-muted-foreground uppercase">{t("designContract.description", { defaultValue: "Description" })}</Label>
          <Input className="h-8 text-xs" value={contractDesc} onChange={(e) => setContractDesc(e.target.value)} />
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList className="grid w-full grid-cols-4 h-8">
          <TabsTrigger value="templates" className="text-[10px] h-6"><Paintbrush className="h-3 w-3 mr-1" />{t("designContract.templates", { defaultValue: "Templates" })}</TabsTrigger>
          <TabsTrigger value="tokens" className="text-[10px] h-6"><Palette className="h-3 w-3 mr-1" />{t("designContract.tokens", { defaultValue: "Tokens" })}</TabsTrigger>
          <TabsTrigger value="preview" className="text-[10px] h-6"><Eye className="h-3 w-3 mr-1" />{t("designContract.preview", { defaultValue: "Preview" })}</TabsTrigger>
          <TabsTrigger value="import" className="text-[10px] h-6"><Upload className="h-3 w-3 mr-1" />{t("designContract.import", { defaultValue: "Import" })}</TabsTrigger>
        </TabsList>

        {/* Templates Tab */}
        <TabsContent value="templates" className="mt-3">
          <div className="grid grid-cols-4 gap-2">
            {packages.map((tpl) => (
              <Card
                key={tpl.id}
                className={cn("cursor-pointer transition-all hover:scale-[1.02] border-2", selectedTemplate === tpl.id ? "border-primary shadow-md" : "border-transparent")}
                onClick={() => loadTemplate(tpl.id)}
              >
                <CardContent className="p-2 text-center">
                  <span className="text-[10px] font-medium">{tpl.name}</span>
                  <span className="block text-[9px] text-muted-foreground">{tpl.applicableScenarios.join(" / ")}</span>
                </CardContent>
              </Card>
            ))}
          </div>
          {loading && <div className="flex items-center gap-2 mt-3 text-xs text-muted-foreground">{LoadIcon}{t("designContract.loadingTemplate", { defaultValue: "Loading template..." })}</div>}
          {packageDetail && (
            <Card className="mt-3 border-primary/25 bg-primary/[0.03]">
              <CardContent className="p-3 space-y-3">
                <div className="flex items-center justify-between"><span className="text-xs font-semibold">{packageDetail.package.name} · 原型能力</span><Badge variant="outline">{packageDetail.package.licenseId}</Badge></div>
                <div><span className="text-[10px] text-muted-foreground">页面/流程模板</span><div className="flex flex-wrap gap-1 mt-1">{packageDetail.components.pageTemplates?.map((page) => <Button key={page} size="sm" variant={selectedPrototype === page ? "secondary" : "outline"} onClick={() => setSelectedPrototype(page)} className="h-6 text-[10px]">{page}</Button>)}</div></div>
                <div className="grid grid-cols-2 gap-3 text-[10px]"><div><span className="text-muted-foreground">组件</span><p>{packageDetail.components.components?.join(" · ")}</p></div><div><span className="text-muted-foreground">主题/响应式</span><p>{packageDetail.responsive.modes?.join(" / ")} · {packageDetail.responsive.rules?.join("；")}</p></div></div>
                <div className="rounded-md border bg-background p-3">
                  <span className="text-[10px] text-muted-foreground">静态原型预览 · {selectedPrototype ?? "选择页面模板"}</span>
                  <PrototypePreview page={selectedPrototype} components={packageDetail.components.components} index={prototypeIndex} colors={colors} fontFamily={fontBase} spacing={spacingBase} />
                  <div className="mt-2 flex items-center justify-between"><p className="text-[10px] text-muted-foreground">{packageDetail.package.applicableScenarios.join(" / ")}</p><Button size="sm" className="text-xs" onClick={() => setActiveTab("preview")}>查看落地说明</Button></div>
                </div>
                <p className="text-[10px] text-muted-foreground">{packageDetail.accessibility.replace(/^# .*\n+/, "")}</p>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        {/* Tokens Tab */}
        <TabsContent value="tokens" className="mt-3 space-y-4">
          {/* Colors */}
          <div className="space-y-2">
            <Label className="text-[10px] font-semibold uppercase text-muted-foreground">{t("designContract.colors", { defaultValue: "Colors" })}</Label>
            <div className="grid grid-cols-2 gap-2">
              {COLOR_FIELDS.map(({ key, label }) => (
                <div key={key} className="flex items-center gap-2">
                  <div className="h-6 w-6 rounded border border-border shrink-0" style={{ backgroundColor: (colors?.[key] as string) ?? "#ccc" }} />
                  <div className="flex-1 space-y-0.5">
                    <span className="text-[9px] text-muted-foreground">{label}</span>
                    <Input className="h-6 text-[10px] px-1.5 font-mono" value={(colors?.[key] as string) ?? ""} onChange={(e) => updateColor(key, e.target.value)} placeholder="#000000" />
                  </div>
                </div>
              ))}
            </div>
          </div>
          {/* Typography */}
          <div className="space-y-2">
            <Label className="text-[10px] font-semibold uppercase text-muted-foreground">{t("designContract.typography", { defaultValue: "Typography" })}</Label>
            <div className="grid grid-cols-3 gap-2">
              <div className="space-y-0.5">
                <span className="text-[9px] text-muted-foreground">Base</span>
                <Input className="h-7 text-[10px]" value={fontBase} onChange={(e) => setFontBase(e.target.value)} />
              </div>
              <div className="space-y-0.5">
                <span className="text-[9px] text-muted-foreground">Heading</span>
                <Input className="h-7 text-[10px]" value={fontHeading} onChange={(e) => setFontHeading(e.target.value)} />
              </div>
              <div className="space-y-0.5">
                <span className="text-[9px] text-muted-foreground">Mono</span>
                <Input className="h-7 text-[10px]" value={fontMono} onChange={(e) => setFontMono(e.target.value)} />
              </div>
            </div>
          </div>
          {/* Spacing */}
          <div className="flex items-center gap-3">
            <Label className="text-[10px] font-semibold uppercase text-muted-foreground">{t("designContract.spacing", { defaultValue: "Spacing Base (px)" })}</Label>
            <Input type="number" className="h-7 w-20 text-xs" value={spacingBase} onChange={(e) => setSpacingBase(Number(e.target.value) || 4)} min={1} max={16} />
          </div>
        </TabsContent>

        {/* Preview Tab */}
        <TabsContent value="preview" className="mt-3">
          {previewMd ? (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-semibold uppercase text-muted-foreground">DESIGN.md</span>
                <Button size="sm" variant="ghost" className="h-5 px-1.5 text-[10px]" onClick={handleCopy}><Copy className="h-3 w-3 mr-1" />{t("designContract.copy", { defaultValue: "Copy" })}</Button>
              </div>
              <pre className="text-[10px] leading-relaxed bg-muted/50 border border-border/50 rounded-lg p-3 overflow-x-auto max-h-[400px] overflow-y-auto whitespace-pre-wrap break-words font-mono">{previewMd}</pre>
            </div>
          ) : (
            <div className="flex flex-col items-center gap-2 py-8 text-muted-foreground">
              <FileText className="h-8 w-8 opacity-40" />
              <span className="text-xs">{t("designContract.noPreview", { defaultValue: "Click Preview to generate DESIGN.md" })}</span>
            </div>
          )}
        </TabsContent>

        {/* Import Tab */}
        <TabsContent value="import" className="mt-3 space-y-4">
          <div className="space-y-2">
            <Label className="text-[10px] font-semibold uppercase text-muted-foreground">{t("designContract.importFile", { defaultValue: "Import from File" })}</Label>
            <Button size="sm" variant="outline" className="w-full text-xs" onClick={handleImportFile} disabled={importing}>
              {importing ? LoadIcon : <Upload className="h-3.5 w-3.5 mr-1" />}
              {t("designContract.pickFile", { defaultValue: "Pick DESIGN.md file..." })}
            </Button>
          </div>
          <div className="space-y-2">
            <Label className="text-[10px] font-semibold uppercase text-muted-foreground">{t("designContract.importUrl", { defaultValue: "导入可信远程设计规范" })}</Label>
            <div className="flex gap-1">
              {(["github", "shadcn"] as const).map((kind) => (
                <Button key={kind} size="sm" variant={remoteImportKind === kind ? "secondary" : "ghost"} className="h-6 px-2 text-[10px]" onClick={() => setRemoteImportKind(kind)}>
                  {kind === "github" ? "GitHub" : "shadcn"}
                </Button>
              ))}
            </div>
            <div className="flex gap-2">
              <Input className="h-8 text-xs flex-1" value={importUrl} onChange={(e) => setImportUrl(e.target.value)} placeholder={remoteImportKind === "github" ? "https://raw.githubusercontent.com/.../DESIGN.md" : "https://ui.shadcn.com/..."} />
              <Button size="sm" variant="outline" className="text-xs shrink-0" onClick={handleImportUrl} disabled={importing || !importUrl.trim()}>
                {importing ? LoadIcon : <Download className="h-3.5 w-3.5" />}
              </Button>
            </div>
          </div>
        </TabsContent>
      </Tabs>

      {/* Action buttons */}
      <div className="flex flex-wrap gap-2 border-t border-border/50 pt-3">
        <Button size="sm" variant="outline" onClick={handlePreview} disabled={previewing} className="text-xs">
          {previewing ? LoadIcon : <Eye className="h-3.5 w-3.5 mr-1" />}
          {t("designContract.previewBtn", { defaultValue: "Preview" })}
        </Button>
        <Button size="sm" variant="outline" onClick={handleCopy} disabled={previewing} className="text-xs">
          <Copy className="h-3.5 w-3.5 mr-1" />{t("designContract.copyBtn", { defaultValue: "Copy" })}
        </Button>
        <Button size="sm" onClick={handleExportPreview} disabled={exporting || !contractName} className="text-xs">
          {exporting ? LoadIcon : <Download className="h-3.5 w-3.5 mr-1" />}
          {t("designContract.exportOverwriteBtn", { defaultValue: "覆盖导出 DESIGN.md" })}
        </Button>
        <Button size="sm" onClick={handleInstallPreview} disabled={installing} className="text-xs">
          {installing ? LoadIcon : <FileText className="h-3.5 w-3.5 mr-1" />}
          {t("designContract.installBtn", { defaultValue: "Install to Project" })}
        </Button>
        <Button size="sm" variant="ghost" onClick={refreshVerification} disabled={verifying} className="text-xs">
          {verifying ? LoadIcon : <Eye className="h-3.5 w-3.5 mr-1" />}
          {t("designContract.verifyBtn", { defaultValue: "验证项目产物" })}
        </Button>
      </div>

      {verification && (
        <Card className={cn(
          "border mt-3",
          verification.state === "verified" ? "border-emerald-500/30 bg-emerald-500/5" : "border-amber-500/30 bg-amber-500/5",
        )}>
          <CardContent className="px-3 py-2 text-xs flex flex-wrap items-center gap-2">
            <Badge variant="outline" className="text-[10px]">
              {verification.state === "verified" ? "项目产物已验证" : verification.state === "missing" ? "未安装设计系统" : verification.state === "invalid" ? "设计系统清单无效" : "项目产物已漂移"}
            </Badge>
            {verification.outputs.map((output) => (
              <span key={output.path} className="font-mono text-[10px] text-muted-foreground">
                {output.path}: {output.state}
              </span>
            ))}
          </CardContent>
        </Card>
      )}

      {/* Install result */}
      {installResult && (
        <Card className="border-emerald-500/30 bg-emerald-500/5">
          <CardHeader className="pb-2 pt-3 px-3">
            <CardTitle className="text-xs font-semibold text-emerald-700 dark:text-emerald-400 flex items-center gap-2">
              <FileText className="h-4 w-4" />
              {t("designContract.installResult", { defaultValue: "Installed" })}: {installResult.filesCreated.length} {t("designContract.filesCreated", { defaultValue: "files created" })}
              {installResult.filesSkipped.length > 0 && `, ${installResult.filesSkipped.length} ${t("designContract.filesSkipped", { defaultValue: "skipped" })}`}
            </CardTitle>
          </CardHeader>
          <CardContent className="px-3 pb-3 pt-0 space-y-0.5">
            {installResult.filesCreated.map((f) => <div key={f} className="text-[10px] text-emerald-600 dark:text-emerald-400 font-mono">+ {f}</div>)}
            {installResult.filesSkipped.map((f) => <div key={f} className="text-[10px] text-muted-foreground font-mono">~ {f} ({t("designContract.exists", { defaultValue: "exists" })})</div>)}
          </CardContent>
        </Card>
      )}

      {/* Pre-flight install confirmation modal */}
      {installPlan && (
        <InstallConfirmModal
          open={!!installPlan}
          files={installPlan.files}
          audit={installPlan.audit}
          onConfirm={handleInstallConfirm}
          onCancel={() => setInstallPlan(null)}
        />
      )}

      {exportPlan && (
        <InstallConfirmModal
          open={!!exportPlan}
          title={t("designContract.confirmExportTitle", {
            defaultValue: "确认覆盖导出设计规范",
          })}
          confirmLabel={t("designContract.confirmExport", {
            defaultValue: "确认覆盖导出",
          })}
          files={exportPlan.files}
          audit={exportPlan.audit}
          onConfirm={handleExportConfirm}
          onCancel={() => setExportPlan(null)}
        />
      )}
    </div>
  );
}
