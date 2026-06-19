import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { CheckCircle, Search, ArrowRight, Sparkles } from "lucide-react";

interface ScanResult {
  providersFound: {
    appType: string;
    name: string;
    configPath: string;
    hasApiKey: boolean;
  }[];
  mcpServersFound: { name: string; sourceApp: string; command?: string }[];
  totalItems: number;
}

interface OnboardingWizardProps {
  onComplete: () => void;
}

export function OnboardingWizard({ onComplete }: OnboardingWizardProps) {
  const [step, setStep] = useState(0);
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [scanning, setScanning] = useState(false);

  const startScan = async () => {
    setScanning(true);
    try {
      const result = await invoke<ScanResult>("scan_environment");
      setScanResult(result);
      setStep(1);
    } catch (e) {
      console.error("Scan failed:", e);
      setStep(1);
    } finally {
      setScanning(false);
    }
  };

  const handleComplete = async () => {
    try {
      await invoke("complete_onboarding");
    } catch (e) {
      console.error("Failed to complete onboarding:", e);
    }
    onComplete();
  };

  if (step === 0) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/95 backdrop-blur-sm">
        <Card className="w-[480px] p-8 space-y-6 shadow-2xl">
          <div className="text-center space-y-3">
            <div className="mx-auto w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center">
              <Sparkles className="w-8 h-8 text-primary" />
            </div>
            <h1 className="text-2xl font-bold">Welcome to OpenSunstar</h1>
            <p className="text-muted-foreground">
              Your unified control panel for AI coding assistants. Let's scan
              your system for existing configurations.
            </p>
          </div>
          <Button
            className="w-full"
            size="lg"
            onClick={startScan}
            disabled={scanning}
          >
            {scanning ? (
              <>
                <Search className="w-4 h-4 mr-2 animate-spin" />
                Scanning...
              </>
            ) : (
              <>
                <Search className="w-4 h-4 mr-2" />
                Scan My System
              </>
            )}
          </Button>
          <Button variant="ghost" className="w-full" onClick={handleComplete}>
            Skip for now
          </Button>
        </Card>
      </div>
    );
  }

  if (step === 1) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/95 backdrop-blur-sm">
        <Card className="w-[520px] p-8 space-y-6 shadow-2xl">
          <div className="text-center space-y-2">
            <h2 className="text-xl font-bold">Scan Results</h2>
            <p className="text-sm text-muted-foreground">
              Found {scanResult?.totalItems || 0} configurations on your system
            </p>
          </div>

          {scanResult && scanResult.providersFound.length > 0 && (
            <div className="space-y-2">
              <h3 className="text-sm font-medium">Detected Providers</h3>
              {scanResult.providersFound.map((p, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 p-3 rounded-lg border"
                >
                  <CheckCircle className="w-4 h-4 text-green-500 shrink-0" />
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm">{p.name}</div>
                    <div className="text-xs text-muted-foreground truncate">
                      {p.configPath}
                    </div>
                  </div>
                  {p.hasApiKey && (
                    <span className="text-xs px-2 py-0.5 rounded bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300">
                      API Key
                    </span>
                  )}
                </div>
              ))}
            </div>
          )}

          {scanResult && scanResult.mcpServersFound.length > 0 && (
            <div className="space-y-2">
              <h3 className="text-sm font-medium">Detected MCP Servers</h3>
              {scanResult.mcpServersFound.map((m, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 p-3 rounded-lg border"
                >
                  <CheckCircle className="w-4 h-4 text-blue-500 shrink-0" />
                  <div className="flex-1">
                    <div className="font-medium text-sm">{m.name}</div>
                    <div className="text-xs text-muted-foreground">
                      from {m.sourceApp}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {(!scanResult || scanResult.totalItems === 0) && (
            <div className="text-center py-6 text-muted-foreground">
              <p>No existing AI tool configurations found.</p>
              <p className="text-sm mt-1">
                You can add providers manually from the settings.
              </p>
            </div>
          )}

          <Button className="w-full" size="lg" onClick={handleComplete}>
            Get Started <ArrowRight className="w-4 h-4 ml-2" />
          </Button>
        </Card>
      </div>
    );
  }

  return null;
}
