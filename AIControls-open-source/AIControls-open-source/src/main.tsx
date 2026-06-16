import { Component, StrictMode, type ErrorInfo, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import FloatBallApp from "./FloatBallApp";
import { I18nProvider } from "./i18n/provider";
import "./styles.css";

class RootErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Root render failed", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 24, fontFamily: "system-ui, sans-serif", color: "#111" }}>
          <h2 style={{ margin: "0 0 12px" }}>App Render Error</h2>
          <pre style={{ whiteSpace: "pre-wrap", lineHeight: 1.5 }}>
            {this.state.error.stack || this.state.error.message}
          </pre>
        </div>
      );
    }
    return this.props.children;
  }
}

function renderBootstrapError(error: unknown) {
  const message = error instanceof Error ? error.stack || error.message : String(error);
  const root = document.getElementById("root");
  if (root) {
    root.innerHTML = `<div style="padding:24px;font-family:system-ui,sans-serif;color:#111"><h2 style="margin:0 0 12px">Bootstrap Error</h2><pre style="white-space:pre-wrap;line-height:1.5">${message.replace(/[<>&]/g, (s) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;' }[s] as string))}</pre></div>`;
  }
}

let isFloatBallWindow = false;
try {
  const currentWindow = getCurrentWindow();
  isFloatBallWindow = currentWindow.label === "float-ball";
} catch (error) {
  console.error("Window bootstrap failed", error);
}

try {
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <RootErrorBoundary>
        {isFloatBallWindow ? (
          <FloatBallApp />
        ) : (
          <I18nProvider>
            <BrowserRouter>
              <App />
            </BrowserRouter>
          </I18nProvider>
        )}
      </RootErrorBoundary>
    </StrictMode>,
  );
} catch (error) {
  console.error("Root bootstrap failed", error);
  renderBootstrapError(error);
}
