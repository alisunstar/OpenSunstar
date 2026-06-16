import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "react-router-dom";

/**
 * Rendered inside the main window only.
 * Listens for "main-navigate" events from the switcher popup and navigates accordingly.
 */
export function GlobalProjectSwitcher() {
  const navigate = useNavigate();

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    listen<string>("main-navigate", (event) => {
      const path = event.payload;
      if (path) {
        navigate(`/project?path=${encodeURIComponent(path)}`);
      }
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, [navigate]);

  return null;
}
