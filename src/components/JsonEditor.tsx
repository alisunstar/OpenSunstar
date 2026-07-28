import React, { useRef, useEffect, useMemo } from "react";
import { EditorView, basicSetup } from "codemirror";
import { json } from "@codemirror/lang-json";
import { javascript } from "@codemirror/lang-javascript";
import { oneDark } from "@codemirror/theme-one-dark";
import { Compartment, EditorState } from "@codemirror/state";
import { placeholder } from "@codemirror/view";
import { linter, Diagnostic } from "@codemirror/lint";
import { useTranslation } from "react-i18next";
import { Wand2 } from "lucide-react";
import { toast } from "sonner";
import { formatJSON } from "@/utils/formatters";
import { useIsDark } from "@/components/theme-provider";

// 暗色模式下的强制覆盖层：保证边框/ gutter 等读设计 token，不被 oneDark 吃掉
const darkOverrideTheme = EditorView.theme({
  ".cm-editor": {
    border: "1px solid hsl(var(--border))",
    borderRadius: "0.5rem",
    background: "transparent",
  },
  ".cm-editor.cm-focused": {
    outline: "none",
    borderColor: "hsl(var(--primary))",
  },
  ".cm-scroller": {
    background: "transparent",
  },
  ".cm-gutters": {
    background: "transparent",
    borderRight: "1px solid hsl(var(--border))",
    color: "hsl(var(--muted-foreground))",
  },
  ".cm-selectionBackground, .cm-content ::selection": {
    background: "hsl(var(--primary) / 0.18)",
  },
  ".cm-selectionMatch": {
    background: "hsl(var(--primary) / 0.12)",
  },
  ".cm-activeLine": {
    background: "hsl(var(--primary) / 0.08)",
  },
  ".cm-activeLineGutter": {
    background: "hsl(var(--primary) / 0.08)",
  },
});

interface JsonEditorProps {
  id?: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  rows?: number;
  showValidation?: boolean;
  language?: "json" | "javascript";
  height?: string | number;
  showMinimap?: boolean; // 添加此属性以防未来使用
}

const JsonEditor: React.FC<JsonEditorProps> = ({
  value,
  onChange,
  placeholder: placeholderText = "",
  rows = 12,
  showValidation = true,
  language = "json",
  height,
}) => {
  const { t } = useTranslation();
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  // 主题从 ThemeProvider 自取，不再依赖调用方 prop drilling
  const darkMode = useIsDark();
  // 主题扩展用 Compartment 包裹：切换时 reconfigure 热替换（A1），
  // 不重建编辑器，光标/撤销历史/滚动位置全部保留
  const themeCompartment = useRef(new Compartment());

  // JSON linter 函数
  const jsonLinter = useMemo(
    () =>
      linter((view) => {
        const diagnostics: Diagnostic[] = [];
        if (!showValidation || language !== "json") return diagnostics;

        const doc = view.state.doc.toString();
        if (!doc.trim()) return diagnostics;

        try {
          const parsed = JSON.parse(doc);
          // 检查是否是JSON对象
          if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
            // 格式正确
          } else {
            diagnostics.push({
              from: 0,
              to: doc.length,
              severity: "error",
              message: t("jsonEditor.mustBeObject"),
            });
          }
        } catch (e) {
          // 简单处理JSON解析错误
          const message =
            e instanceof SyntaxError ? e.message : t("jsonEditor.invalidJson");
          diagnostics.push({
            from: 0,
            to: doc.length,
            severity: "error",
            message,
          });
        }

        return diagnostics;
      }),
    [showValidation, language, t],
  );

  useEffect(() => {
    if (!editorRef.current) return;

    // 创建编辑器扩展
    const minHeightPx = height ? undefined : Math.max(1, rows) * 18;

    // 使用 baseTheme 定义基础样式，优先级低于 oneDark，但可以正确响应主题
    const baseTheme = EditorView.baseTheme({
      ".cm-editor": {
        border: "1px solid hsl(var(--border))",
        borderRadius: "0.5rem",
        background: "transparent",
      },
      ".cm-editor.cm-focused": {
        outline: "none",
        borderColor: "hsl(var(--primary))",
      },
      ".cm-scroller": {
        background: "transparent",
      },
      ".cm-gutters": {
        background: "transparent",
        borderRight: "1px solid hsl(var(--border))",
        color: "hsl(var(--muted-foreground))",
      },
      ".cm-selectionBackground, .cm-content ::selection": {
        background: "hsl(var(--primary) / 0.18)",
      },
      ".cm-selectionMatch": {
        background: "hsl(var(--primary) / 0.12)",
      },
      ".cm-activeLine": {
        background: "hsl(var(--primary) / 0.08)",
      },
      ".cm-activeLineGutter": {
        background: "hsl(var(--primary) / 0.08)",
      },
    });

    // 使用 theme 定义尺寸和字体样式
    const heightValue = height
      ? typeof height === "number"
        ? `${height}px`
        : height
      : undefined;
    const sizingTheme = EditorView.theme({
      "&": heightValue
        ? { height: heightValue }
        : { minHeight: `${minHeightPx}px` },
      ".cm-scroller": { overflow: "auto" },
      ".cm-content": {
        fontFamily:
          "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace",
        fontSize: "14px",
      },
    });

    const extensions = [
      basicSetup,
      language === "javascript" ? javascript() : json(),
      placeholder(placeholderText || ""),
      baseTheme,
      sizingTheme,
      jsonLinter,
      // 暗色主题经 compartment 注入，切换时热替换不重建编辑器
      themeCompartment.current.of(darkMode ? [oneDark, darkOverrideTheme] : []),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          const newValue = update.state.doc.toString();
          onChange(newValue);
        }
      }),
    ];

    // 创建初始状态
    const state = EditorState.create({
      doc: value,
      extensions,
    });

    // 创建编辑器视图
    const view = new EditorView({
      state,
      parent: editorRef.current,
    });

    viewRef.current = view;

    // 清理函数
    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // darkMode 不在依赖中：主题切换走下方 reconfigure，编辑器不重建
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, height, language, jsonLinter]); // 依赖项中不包含 onChange 和 placeholder，避免不必要的重建

  // 主题热切换：只重配 compartment，保留输入现场
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: themeCompartment.current.reconfigure(
        darkMode ? [oneDark, darkOverrideTheme] : [],
      ),
    });
  }, [darkMode]);

  // 当 value 从外部改变时更新编辑器内容
  useEffect(() => {
    if (viewRef.current && viewRef.current.state.doc.toString() !== value) {
      const transaction = viewRef.current.state.update({
        changes: {
          from: 0,
          to: viewRef.current.state.doc.length,
          insert: value,
        },
      });
      viewRef.current.dispatch(transaction);
    }
  }, [value]);

  // 格式化处理函数
  const handleFormat = () => {
    if (!viewRef.current) return;

    const currentValue = viewRef.current.state.doc.toString();
    if (!currentValue.trim()) return;

    try {
      const formatted = formatJSON(currentValue);
      onChange(formatted);
      toast.success(t("common.formatSuccess", { defaultValue: "格式化成功" }), {
        closeButton: true,
      });
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      toast.error(
        t("common.formatError", {
          defaultValue: "格式化失败：{{error}}",
          error: errorMessage,
        }),
      );
    }
  };

  const isFullHeight = height === "100%";

  return (
    <div
      style={{ width: "100%", height: isFullHeight ? "100%" : "auto" }}
      className={isFullHeight ? "flex flex-col" : ""}
    >
      <div
        ref={editorRef}
        style={{ width: "100%", height: isFullHeight ? undefined : "auto" }}
        className={isFullHeight ? "flex-1 min-h-0" : ""}
      />
      {language === "json" && (
        <button
          type="button"
          onClick={handleFormat}
          className={`${isFullHeight ? "mt-2 flex-shrink-0" : "mt-2"} inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-muted-foreground hover:text-primary transition-colors`}
        >
          <Wand2 className="w-3.5 h-3.5" />
          {t("common.format", { defaultValue: "格式化" })}
        </button>
      )}
    </div>
  );
};

export default JsonEditor;
