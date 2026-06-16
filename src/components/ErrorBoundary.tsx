import { Component, type ReactNode } from "react";
import { AlertTriangle, RefreshCw, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** 自定义回退时的标题 */
  fallbackTitle?: string;
  /** 自定义回退时的描述 */
  fallbackDescription?: string;
  /** 返回按钮的回调（如切换回上一视图） */
  onGoBack?: () => void;
  /** 重置后的回调 */
  onReset?: () => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error(
      "[ErrorBoundary] 捕获到组件渲染错误:",
      error.message,
      "\n组件栈:",
      errorInfo.componentStack,
    );
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
    this.props.onReset?.();
  };

  handleGoBack = () => {
    this.setState({ hasError: false, error: null });
    this.props.onGoBack?.();
  };

  render() {
    if (this.state.hasError) {
      const title = this.props.fallbackTitle ?? "页面加载失败";
      const description =
        this.props.fallbackDescription ??
        "渲染过程中发生了未预期的错误，请尝试刷新或返回上一页。";

      return (
        <div className="flex flex-col items-center justify-center flex-1 min-h-0 px-6 py-16">
          <div className="w-16 h-16 rounded-full bg-destructive/10 flex items-center justify-center mb-4">
            <AlertTriangle size={28} className="text-destructive" />
          </div>
          <h3 className="text-lg font-semibold text-foreground mb-2">
            {title}
          </h3>
          <p className="text-sm text-muted-foreground text-center max-w-md mb-2">
            {description}
          </p>
          {this.state.error && (
            <details className="mb-6 max-w-lg w-full">
              <summary className="text-xs text-muted-foreground/60 cursor-pointer hover:text-muted-foreground select-none">
                查看错误详情
              </summary>
              <pre className="mt-2 p-3 rounded-lg bg-muted text-xs text-muted-foreground overflow-auto max-h-40 whitespace-pre-wrap break-all">
                {this.state.error.message}
                {this.state.error.stack ? `\n\n${this.state.error.stack}` : ""}
              </pre>
            </details>
          )}
          <div className="flex items-center gap-3">
            {this.props.onGoBack && (
              <Button
                variant="outline"
                size="sm"
                onClick={this.handleGoBack}
              >
                <ArrowLeft size={14} className="mr-1" />
                返回
              </Button>
            )}
            <Button
              variant="default"
              size="sm"
              onClick={this.handleReset}
            >
              <RefreshCw size={14} className="mr-1" />
              重试
            </Button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
