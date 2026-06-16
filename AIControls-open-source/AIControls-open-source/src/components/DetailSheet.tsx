import { useEffect } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";

interface DetailSheetProps {
  open: boolean;
  title: ReactNode;
  description?: ReactNode;
  meta?: ReactNode;
  onClose: () => void;
  children: ReactNode;
}

export function DetailSheet({
  open,
  title,
  description,
  meta,
  onClose,
  children,
}: DetailSheetProps) {
  useEffect(() => {
    if (!open) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <>
      <div
        className="detail-sheet-sidebar-dismiss"
        onClick={onClose}
        aria-hidden
      />

      <div className="detail-sheet-root">
        <div className="detail-sheet-backdrop" onClick={onClose} aria-hidden />

        <div className="detail-sheet-panel">
        <button
          type="button"
          onClick={onClose}
          className="detail-sheet-close"
          aria-label="关闭"
        >
          ✕
        </button>

        <div className="detail-sheet-body">
          <h2 className="detail-sheet-title">{title}</h2>

          {description ? (
            <div className="detail-sheet-desc">{description}</div>
          ) : null}

          {meta ? <div className="detail-sheet-meta">{meta}</div> : null}

          <div className="detail-sheet-children">{children}</div>
        </div>
      </div>
    </div>
    </>,
    document.body,
  );
}
