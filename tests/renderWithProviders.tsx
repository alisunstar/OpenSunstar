import {
  Suspense,
  type ComponentType,
  type ReactElement,
  type ReactNode,
} from "react";
import { render, type RenderOptions } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { I18nextProvider } from "react-i18next";
import i18n from "i18next";
import { ThemeProvider } from "@/components/theme-provider";

export interface RenderWithProvidersOptions
  extends Omit<RenderOptions, "wrapper"> {
  queryClient?: QueryClient;
  theme?: "light" | "dark" | "system";
}

export function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
}

function TestProviders({
  children,
  queryClient,
  theme = "light",
}: {
  children: ReactNode;
  queryClient: QueryClient;
  theme?: "light" | "dark" | "system";
}) {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider defaultTheme={theme} storageKey="OpenSunstar-test-theme">
        <I18nextProvider i18n={i18n}>{children}</I18nextProvider>
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export function renderWithProviders(
  ui: ReactElement,
  options?: RenderWithProvidersOptions,
) {
  const queryClient = options?.queryClient ?? createTestQueryClient();
  const theme = options?.theme ?? "light";

  return render(ui, {
    wrapper: ({ children }) => (
      <TestProviders queryClient={queryClient} theme={theme}>
        {children}
      </TestProviders>
    ),
    ...options,
  });
}

/** Matches `main.tsx` provider stack for App-level integration tests. */
export function renderAppWithProviders(
  AppComponent: ComponentType,
  options?: RenderWithProvidersOptions,
) {
  return renderWithProviders(
    <Suspense fallback={<div data-testid="loading">loading</div>}>
      <AppComponent />
    </Suspense>,
    options,
  );
}
