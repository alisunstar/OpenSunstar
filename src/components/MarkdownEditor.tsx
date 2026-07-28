import React, { useRef, useEffect } from "react";
import { EditorView, basicSetup } from "codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { oneDark } from "@codemirror/theme-one-dark";
import { Compartment, EditorState } from "@codemirror/state";
import { placeholder as placeholderExt } from "@codemirror/view";
import { useIsDark } from "@/components/theme-provider";

interface MarkdownEditorProps {
  value: string;
  onChange?: (value: string) => void;
  placeholder?: string;
  readOnly?: boolean;
  className?: string;
  minHeight?: string;
  maxHeight?: string;
}

// 浅色主题：全部读设计 token，正文色柔化避免纯黑刺眼（A4）
const lightTheme = EditorView.theme(
  {
    "&": {
      backgroundColor: "transparent",
    },
    ".cm-content": {
      color: "hsl(var(--foreground) / 0.85)",
    },
    ".cm-gutters": {
      backgroundColor: "hsl(var(--muted))",
      color: "hsl(var(--muted-foreground))",
      borderRight: "1px solid hsl(var(--border))",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "hsl(var(--accent))",
    },
  },
  { dark: false },
);

const MarkdownEditor: React.FC<MarkdownEditorProps> = ({
  value,
  onChange,
  placeholder: placeholderText = "",
  readOnly = false,
  className = "",
  minHeight = "300px",
  maxHeight,
}) => {
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  // 主题从 ThemeProvider 自取，不再依赖调用方 prop drilling
  const darkMode = useIsDark();
  // 主题扩展用 Compartment 包裹：切换时 reconfigure 热替换（A1），
  // 不重建编辑器，光标/撤销历史/滚动位置全部保留
  const themeCompartment = useRef(new Compartment());

  useEffect(() => {
    if (!editorRef.current) return;

    // 定义基础主题
    const baseTheme = EditorView.baseTheme({
      "&": {
        height: "100%",
        minHeight,
        maxHeight: maxHeight || "none",
      },
      ".cm-scroller": {
        overflow: "auto",
        fontFamily:
          "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace",
        fontSize: "14px",
      },
      "&light .cm-content, &dark .cm-content": {
        padding: "12px 0",
      },
      "&light .cm-editor, &dark .cm-editor": {
        backgroundColor: "transparent",
      },
      "&.cm-focused": {
        outline: "none",
      },
    });

    const extensions = [
      basicSetup,
      markdown(),
      baseTheme,
      EditorView.lineWrapping,
      EditorState.readOnly.of(readOnly),
      themeCompartment.current.of(darkMode ? oneDark : lightTheme),
    ];

    if (!readOnly) {
      extensions.push(
        placeholderExt(placeholderText),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && onChange) {
            onChange(update.state.doc.toString());
          }
        }),
      );
    } else {
      // 只读模式下隐藏光标和高亮行
      extensions.push(
        EditorView.theme({
          ".cm-cursor, .cm-dropCursor": { border: "none" },
          ".cm-activeLine": { backgroundColor: "transparent !important" },
          ".cm-activeLineGutter": { backgroundColor: "transparent !important" },
        }),
      );
    }

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

    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // darkMode 不在依赖中：主题切换走下方 reconfigure，编辑器不重建
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [readOnly, minHeight, maxHeight, placeholderText]);

  // 主题热切换：只重配 compartment，保留输入现场
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: themeCompartment.current.reconfigure(
        darkMode ? oneDark : lightTheme,
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

  return (
    <div
      ref={editorRef}
      className={`border border-input rounded-md overflow-hidden ${className}`}
    />
  );
};

export default MarkdownEditor;
