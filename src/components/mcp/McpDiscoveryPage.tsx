import {
  useState,
  useMemo,
  forwardRef,
  useImperativeHandle,
  useCallback,
} from "react";
import { useTranslation } from "react-i18next";
import { Search, Loader2, RefreshCw, ChevronLeft, ChevronRight, AlertTriangle } from "lucide-react";
// ChevronRight used as ChevronNext below
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { McpDiscoveryCard } from "./McpDiscoveryCard";
import { useRegistryServers } from "@/hooks/useMcpDiscovery";
import { useAllMcpServers } from "@/hooks/useMcp";

export interface McpDiscoveryPageHandle {
  refresh: () => void;
}

const PAGE_SIZE = 60;

export const McpDiscoveryPage = forwardRef<McpDiscoveryPageHandle, object>(
  (_props, ref) => {
    const { t } = useTranslation();
    const [searchInput, setSearchInput] = useState("");
    const [searchQuery, setSearchQuery] = useState("");
    const [cursorStack, setCursorStack] = useState<string[]>([]);
    const [currentCursor, setCurrentCursor] = useState<string | undefined>(undefined);

    const {
      data,
      isLoading,
      isError,
      error,
      isFetching,
      refetch,
    } = useRegistryServers(
      searchQuery || undefined,
      currentCursor,
      PAGE_SIZE,
    );

    const { data: existingServers } = useAllMcpServers();

    useImperativeHandle(ref, () => ({
      refresh: () => refetch(),
    }));

    const handleSearch = () => {
      setSearchQuery(searchInput.trim());
      setCursorStack([]);
      setCurrentCursor(undefined);
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        handleSearch();
      }
    };

    const filteredServers = useMemo(() => {
      if (!data?.servers) return [];
      if (!searchQuery) return data.servers;
      const q = searchQuery.toLowerCase();
      return data.servers.filter((entry) => {
        const s = entry.server;
        return (
          s.name.toLowerCase().includes(q) ||
          (s.title && s.title.toLowerCase().includes(q)) ||
          (s.description && s.description.toLowerCase().includes(q)) ||
          (s.tags && s.tags.some((tag: string) => tag.toLowerCase().includes(q)))
        );
      });
    }, [data, searchQuery]);

    const handleNextPage = useCallback(() => {
      if (data?.metadata?.nextCursor) {
        setCursorStack((prev) => [...prev, currentCursor || ""]);
        setCurrentCursor(data.metadata.nextCursor);
      }
    }, [data, currentCursor]);

    const handlePrevPage = useCallback(() => {
      setCursorStack((prev) => {
        const newStack = [...prev];
        const prevCursor = newStack.pop();
        setCurrentCursor(prevCursor || undefined);
        return newStack;
      });
    }, []);

    const installedCount = useMemo(() => {
      if (!existingServers) return 0;
      let count = 0;
      filteredServers.forEach((entry) => {
        const id = entry.server.name
          .replace(/[/@.]/g, "-")
          .replace(/^-+/, "")
          .replace(/-+$/, "");
        if (id in existingServers) count++;
      });
      return count;
    }, [filteredServers, existingServers]);

    const totalCount = data?.metadata?.count ?? filteredServers.length;

    return (
      <div className="flex flex-col flex-1 min-h-0 overflow-hidden px-6">
        {/* 搜索栏 */}
        <div className="flex items-center gap-3 py-3 flex-shrink-0">
          <div className="flex-1 relative">
            <Search
              size={16}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
            />
            <Input
              type="text"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t("mcp.discovery.searchPlaceholder", {
                defaultValue: "输入关键字搜索 MCP 服务器...",
              })}
              className="pl-9"
            />
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleSearch}
            disabled={isFetching}
          >
            <Search size={14} className="mr-1" />
            {t("common.search", { defaultValue: "搜索" })}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => refetch()}
            disabled={isFetching}
            title={t("common.refresh", { defaultValue: "刷新" })}
          >
            <RefreshCw size={14} className={isFetching ? "animate-spin" : ""} />
          </Button>
        </div>

        {/* 统计栏 */}
        <div className="flex items-center gap-4 pb-3 flex-shrink-0">
          <p className="text-sm text-muted-foreground">
            {t("mcp.discovery.totalFound", {
              defaultValue: `找到 {{count}} 个服务器`,
              count: totalCount,
            })}
            {installedCount > 0 &&
              ` · ${t("mcp.discovery.alreadyInstalled", {
                defaultValue: `已安装 {{count}} 个`,
                count: installedCount,
              })}`}
          </p>
        </div>

        {/* 卡片网格 */}
        <div className="flex-1 overflow-y-auto overflow-x-hidden pb-24">
          {isLoading ? (
            <div className="flex items-center justify-center py-16">
              <Loader2 size={24} className="animate-spin text-muted-foreground" />
              <span className="ml-2 text-sm text-muted-foreground">
                {t("common.loading", { defaultValue: "加载中..." })}
              </span>
            </div>
          ) : isError ? (
            <div className="flex flex-col items-center justify-center py-16">
              <div className="w-14 h-14 rounded-full bg-destructive/10 flex items-center justify-center mb-3">
                <AlertTriangle size={24} className="text-destructive" />
              </div>
              <p className="text-sm font-medium text-foreground mb-1">
                {t("mcp.discovery.loadError", { defaultValue: "加载 MCP 注册表失败" })}
              </p>
              <p className="text-xs text-muted-foreground text-center max-w-sm mb-4">
                {error instanceof Error
                  ? error.message
                  : String(error ?? t("common.unknownError", { defaultValue: "未知错误" }))}
              </p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => refetch()}
                disabled={isFetching}
              >
                <RefreshCw size={14} className={isFetching ? "animate-spin mr-1" : "mr-1"} />
                {t("common.retry", { defaultValue: "重试" })}
              </Button>
            </div>
          ) : filteredServers.length === 0 ? (
            <div className="text-center py-16">
              <Search size={32} className="mx-auto mb-3 text-muted-foreground/40" />
              <p className="text-sm text-muted-foreground">
                {searchQuery
                  ? t("mcp.discovery.noResults", {
                      defaultValue: `未找到与 "${searchQuery}" 相关的服务器`,
                      query: searchQuery,
                    })
                  : t("mcp.discovery.noServers", {
                      defaultValue: "暂无可用服务器",
                    })}
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {filteredServers.map((entry) => (
                <McpDiscoveryCard
                  key={entry.server.name}
                  server={entry.server}
                  meta={entry._meta as Record<string, unknown> | undefined}
                />
              ))}
            </div>
          )}
        </div>

        {/* 分页 */}
        {data?.metadata && (
          <div className="flex items-center justify-between py-3 flex-shrink-0 border-t border-border-default">
            <Button
              variant="ghost"
              size="sm"
              onClick={handlePrevPage}
              disabled={cursorStack.length === 0}
            >
              <ChevronLeft size={14} className="mr-1" />
              {t("common.previous", { defaultValue: "上一页" })}
            </Button>
            <span className="text-xs text-muted-foreground">
              {cursorStack.length + 1}
            </span>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleNextPage}
              disabled={!data.metadata.nextCursor}
            >
              {t("common.next", { defaultValue: "下一页" })}
              <ChevronRight size={14} className="ml-1" />
            </Button>
          </div>
        )}
      </div>
    );
  },
);

McpDiscoveryPage.displayName = "McpDiscoveryPage";
