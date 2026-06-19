import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { FileDown, FileJson, FileSpreadsheet, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

interface ExportMenuProps {
  appType: string;
  startDate?: number;
  endDate?: number;
}

interface ExportResult {
  content: string;
  suggestedFilename: string;
}

export function ExportMenu({ appType, startDate, endDate }: ExportMenuProps) {
  const [exporting, setExporting] = useState(false);

  const handleExport = async (format: "csv" | "json") => {
    setExporting(true);
    try {
      const result = await invoke<ExportResult>("export_usage", {
        request: {
          appType: appType === "all" ? null : appType,
          startDate: startDate ?? null,
          endDate: endDate ?? null,
          format,
        },
      });

      const blob = new Blob([result.content], {
        type: format === "csv" ? "text/csv;charset=utf-8" : "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = result.suggestedFilename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      toast.success(`Exported as ${result.suggestedFilename}`);
    } catch (e) {
      toast.error(`Export failed: ${e}`);
    } finally {
      setExporting(false);
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="h-9 px-3 text-xs" disabled={exporting}>
          {exporting ? (
            <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
          ) : (
            <FileDown className="mr-2 h-3.5 w-3.5" />
          )}
          Export
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onClick={() => handleExport("csv")}>
          <FileSpreadsheet className="w-4 h-4 mr-2" />
          Export as CSV
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => handleExport("json")}>
          <FileJson className="w-4 h-4 mr-2" />
          Export as JSON
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
