import {
  useState,
  useMemo,
  forwardRef,
  useImperativeHandle,
  useCallback,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Search,
  Loader2,
  RefreshCw,
  ChevronLeft,
  ChevronRight,
  AlertTriangle,
  Flame,
  Star,
  SearchIcon,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { McpDiscoveryCard } from "./McpDiscoveryCard";
import { McpSmitheryCard } from "./McpSmitheryCard";
import {
  useRegistryServers,
  useSmitheryServers,
} from "@/hooks/useMcpDiscovery";
import { useAllMcpServers } from "@/hooks/useMcp";

export interface McpDiscoveryPageHandle {
  refresh: () => void;
}

const SMITHERY_PAGE_SIZE = 50;

// ═══════════════════════════════════════════════════════════════
// 主页面：Tabs 容器（条件渲染实现懒加载，仅激活 Tab 发请求）
// ═══════════════════════════════════════════════════════════════

export const McpDiscoveryPage = forwardRef<McpDiscoveryPageHandle, object>(
  (_props, ref) => {
    const { t } = useTranslation();
    const [activeTab, setActiveTab] = useState("search");

    // 每个 Tab 各自管理刷新，外部 handle 暂留空接口
    useImperativeHandle(ref, () => ({
      refresh: () => {},
    }));

    return (
      <div className="flex flex-col flex-1 min-h-0 overflow-hidden px-6">
        <Tabs
          value={activeTab}
          onValueChange={setActiveTab}
          className="flex flex-col flex-1 min-h-0"
        >
          <TabsList className="flex-shrink-0 self-start mt-3 mb-1">
            <TabsTrigger value="search" className="gap-1.5 text-xs min-w-[100px]">
              <SearchIcon size={13} />
              {t("mcp.discovery.tabSearch", { defaultValue: "搜索 MCP" })}
            </TabsTrigger>
            <TabsTrigger value="hot" className="gap-1.5 text-xs min-w-[100px]">
              <Flame size={13} />
              {t("mcp.discovery.tabHot", { defaultValue: "安装热榜" })}
            </TabsTrigger>
            <TabsTrigger value="picks" className="gap-1.5 text-xs min-w-[100px]">
              <Star size={13} />
              {t("mcp.discovery.tabPicks", { defaultValue: "编辑精选" })}
            </TabsTrigger>
          </TabsList>

          {/* 条件渲染：仅挂载激活的 Tab 面板，未激活的不发请求 */}
          {activeTab === "search" && <OfficialRegistryPanel />}
          {activeTab === "hot" && <SmitheryPanel mode="hot" />}
          {activeTab === "picks" && <SmitheryPanel mode="picks" />}
        </Tabs>
      </div>
    );
  },
);

McpDiscoveryPage.displayName = "McpDiscoveryPage";

// ═══════════════════════════════════════════════════════════════
// Tab 1: Official Registry 搜索面板
// ═══════════════════════════════════════════════════════════════

function OfficialRegistryPanel() {
  const { t } = useTranslation();
  const [searchInput, setSearchInput] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [cursorStack, setCursorStack] = useState<string[]>([]);
  const [currentCursor, setCurrentCursor] = useState<string | undefined>(
    undefined,
  );

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
    SMITHERY_PAGE_SIZE,
  );

  const { data: existingServers } = useAllMcpServers();

  const handleSearch = () => {
    setSearchQuery(searchInput.trim());
    setCursorStack([]);
    setCurrentCursor(undefined);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleSearch();
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
    <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
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
              defaultValue:
                "输入关键词搜索所有来源（Official Registry）...",
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
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden">
        {isLoading ? (
          <LoadingState />
        ) : isError ? (
          <ErrorState
            error={error}
            onRetry={() => refetch()}
            isFetching={isFetching}
          />
        ) : filteredServers.length === 0 ? (
          <EmptyState searchQuery={searchQuery} />
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 pb-4">
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
        <PaginationBar
          hasPrev={cursorStack.length > 0}
          hasNext={!!data.metadata.nextCursor}
          currentPage={cursorStack.length + 1}
          onPrev={handlePrevPage}
          onNext={handleNextPage}
        />
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// Tab 2 & 3: Smithery 面板（安装热榜 / 编辑精选）
// ═══════════════════════════════════════════════════════════════

function SmitheryPanel({ mode }: { mode: "hot" | "picks" }) {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);

  const verified = mode === "picks" ? true : undefined;

  const {
    data,
    isLoading,
    isError,
    error,
    isFetching,
    refetch,
  } = useSmitheryServers(page, SMITHERY_PAGE_SIZE, verified);

  const { data: existingServers } = useAllMcpServers();

  const servers = data?.servers ?? [];
  const totalCount = data?.pagination?.totalCount ?? 0;
  const totalPages = data?.pagination?.totalPages ?? 1;

  const installedCount = useMemo(() => {
    if (!existingServers) return 0;
    let count = 0;
    servers.forEach((s) => {
      const id = s.qualifiedName
        .replace(/[/@.]/g, "-")
        .replace(/^-+/, "")
        .replace(/-+$/, "");
      if (id in existingServers) count++;
    });
    return count;
  }, [servers, existingServers]);

  const subtitle =
    mode === "hot"
      ? t("mcp.discovery.hotSubtitle", {
          defaultValue: "按安装量排序的热门 MCP 服务器（Smithery）",
        })
      : t("mcp.discovery.picksSubtitle", {
          defaultValue: "经 Smithery 认证的高质量 MCP 服务器",
        });

  return (
    <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
      {/* 标题栏 */}
      <div className="flex items-center gap-3 py-3 flex-shrink-0">
        <div className="flex-1">
          <p className="text-xs text-muted-foreground">{subtitle}</p>
        </div>
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
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden">
        {isLoading ? (
          <LoadingState />
        ) : isError ? (
          <ErrorState
            error={error}
            onRetry={() => refetch()}
            isFetching={isFetching}
          />
        ) : servers.length === 0 ? (
          <EmptyState searchQuery="" />
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 pb-4">
            {servers.map((server) => (
              <McpSmitheryCard
                key={server.id}
                server={server}
                showUseCount={mode === "hot"}
              />
            ))}
          </div>
        )}
      </div>

      {/* 分页 */}
      {totalPages > 1 && (
        <PaginationBar
          hasPrev={page > 1}
          hasNext={page < totalPages}
          currentPage={page}
          onPrev={() => setPage((p) => Math.max(1, p - 1))}
          onNext={() => setPage((p) => Math.min(totalPages, p + 1))}
        />
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// 共享子组件
// ═══════════════════════════════════════════════════════════════

function LoadingState() {
  const { t } = useTranslation();
  return (
    <div className="flex items-center justify-center py-16">
      <Loader2 size={24} className="animate-spin text-muted-foreground" />
      <span className="ml-2 text-sm text-muted-foreground">
        {t("common.loading", { defaultValue: "加载中..." })}
      </span>
    </div>
  );
}

function ErrorState({
  error,
  onRetry,
  isFetching,
}: {
  error: unknown;
  onRetry: () => void;
  isFetching: boolean;
}) {
  const { t } = useTranslation();
  return (
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
          : String(
              error ?? t("common.unknownError", { defaultValue: "未知错误" }),
            )}
      </p>
      <Button
        variant="outline"
        size="sm"
        onClick={onRetry}
        disabled={isFetching}
      >
        <RefreshCw
          size={14}
          className={isFetching ? "animate-spin mr-1" : "mr-1"}
        />
        {t("common.retry", { defaultValue: "重试" })}
      </Button>
    </div>
  );
}

function EmptyState({ searchQuery }: { searchQuery: string }) {
  const { t } = useTranslation();
  return (
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
  );
}

function PaginationBar({
  hasPrev,
  hasNext,
  currentPage,
  onPrev,
  onNext,
}: {
  hasPrev: boolean;
  hasNext: boolean;
  currentPage: number;
  onPrev: () => void;
  onNext: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex items-center justify-between py-3 flex-shrink-0 border-t border-border-default">
      <Button variant="ghost" size="sm" onClick={onPrev} disabled={!hasPrev}>
        <ChevronLeft size={14} className="mr-1" />
        {t("common.previous", { defaultValue: "上一页" })}
      </Button>
      <span className="text-xs text-muted-foreground">{currentPage}</span>
      <Button variant="ghost" size="sm" onClick={onNext} disabled={!hasNext}>
        {t("common.next", { defaultValue: "下一页" })}
        <ChevronRight size={14} className="ml-1" />
      </Button>
    </div>
  );
}
