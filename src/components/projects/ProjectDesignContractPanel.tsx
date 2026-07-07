import { useCallback, useState } from "react";
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
} from "@/lib/api/designContract";
import { InstallConfirmModal } from "@/components/shared/InstallConfirmModal";

const TEMPLATES = [
  { id: "vercel", name: "Vercel", color: "#000000" },
  { id: "apple", name: "Apple", color: "#555555" },
  { id: "stripe", name: "Stripe", color: "#635BFF" },
  { id: "linear", name: "Linear", color: "#5E6AD2" },
  { id: "notion", name: "Notion", color: "#2F80ED" },
  { id: "github", name: "GitHub", color: "#24292F" },
  { id: "shadcn", name: "shadcn", color: "#18181B" },
  { id: "neutral", name: "Neutral", color: "#737373" },
] as const;

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
  const [importing, setImporting] = useState(false);
  const [installPlan, setInstallPlan] = useState<DesignInstallPlan | null>(null);

  const setters = { setContract, setColors, setFontBase, setFontHeading, setFontMono, setSpacingBase, setContractName, setActiveTab };

  const buildParams = useCallback((): DesignContractParams => ({
    templateId: selectedTemplate,
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
  }), [selectedTemplate, contractName, contractDesc, colors, contract, fontBase, fontHeading, fontMono, spacingBase]);

  const loadTemplate = useCallback(async (id: string) => {
    setLoading(true);
    try {
      applyContract(await designContractApi.getTemplate(id), setters);
      setSelectedTemplate(id);
    } catch (e) { toast.error(String(e)); }
    finally { setLoading(false); }
  }, []);

  const handlePreview = useCallback(async () => {
    setPreviewing(true);
    try { setPreviewMd(await designContractApi.previewDesignMd(buildParams())); setActiveTab("preview"); }
    catch (e) { toast.error(String(e)); }
    finally { setPreviewing(false); }
  }, [buildParams]);

  const handleCopy = useCallback(async () => {
    if (!previewMd) { await handlePreview(); return; }
    try { await navigator.clipboard.writeText(previewMd); toast.success(t("designContract.copied", { defaultValue: "Copied to clipboard" })); }
    catch { toast.error("Failed to copy"); }
  }, [previewMd, handlePreview, t]);

  const handleExport = useCallback(async () => {
    setExporting(true);
    try { const p = await designContractApi.exportContract(projectId, buildParams()); toast.success(t("designContract.exported", { defaultValue: "Exported" })); setPreviewMd(p); }
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
    } catch (e) { toast.error(String(e)); }
    finally { setImporting(false); }
  }, []);

  const handleImportUrl = useCallback(async () => {
    if (!importUrl.trim()) return;
    setImporting(true);
    try {
      const r = await designContractApi.importFromUrl(await (await fetch(importUrl)).text(), importUrl);
      applyContract(r.contract, setters);
      setSelectedTemplate(null);
      setImportUrl("");
      if (r.warnings.length) toast.warning(r.warnings.join(", "));
    } catch (e) { toast.error(String(e)); }
    finally { setImporting(false); }
  }, [importUrl]);

  const updateColor = useCallback((key: keyof DesignColors, value: string) => {
    setColors((prev) => prev ? { ...prev, [key]: value } : prev);
  }, []);

  const LoadIcon = <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />;

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
            {TEMPLATES.map((tpl) => (
              <Card
                key={tpl.id}
                className={cn("cursor-pointer transition-all hover:scale-[1.02] border-2", selectedTemplate === tpl.id ? "border-primary shadow-md" : "border-transparent")}
                onClick={() => loadTemplate(tpl.id)}
              >
                <div className="h-8 rounded-t-lg" style={{ backgroundColor: tpl.color }} />
                <CardContent className="p-2 text-center">
                  <span className="text-[10px] font-medium">{tpl.name}</span>
                </CardContent>
              </Card>
            ))}
          </div>
          {loading && <div className="flex items-center gap-2 mt-3 text-xs text-muted-foreground">{LoadIcon}{t("designContract.loadingTemplate", { defaultValue: "Loading template..." })}</div>}
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
            <Label className="text-[10px] font-semibold uppercase text-muted-foreground">{t("designContract.importUrl", { defaultValue: "Import from URL" })}</Label>
            <div className="flex gap-2">
              <Input className="h-8 text-xs flex-1" value={importUrl} onChange={(e) => setImportUrl(e.target.value)} placeholder="https://raw.githubusercontent.com/.../DESIGN.md" />
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
        <Button size="sm" onClick={handleExport} disabled={exporting || !contractName} className="text-xs">
          {exporting ? LoadIcon : <Download className="h-3.5 w-3.5 mr-1" />}
          {t("designContract.exportBtn", { defaultValue: "Export" })}
        </Button>
        <Button size="sm" onClick={handleInstallPreview} disabled={installing} className="text-xs">
          {installing ? LoadIcon : <FileText className="h-3.5 w-3.5 mr-1" />}
          {t("designContract.installBtn", { defaultValue: "Install to Project" })}
        </Button>
      </div>

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
    </div>
  );
}
