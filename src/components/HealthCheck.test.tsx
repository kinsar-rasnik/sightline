import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, test, vi } from "vitest";

import { HealthCheck } from "@/components/HealthCheck";

import type * as IpcModule from "@/ipc";

vi.mock("@/ipc", async () => {
  const actual = await vi.importActual<typeof IpcModule>("@/ipc");
  return {
    ...actual,
    commands: {
      health: vi.fn(),
    },
  };
});

import { commands } from "@/ipc";

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe("HealthCheck", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test("renders a health report once the command resolves", async () => {
    vi.mocked(commands.health).mockResolvedValue({
      appName: "sightline",
      appVersion: "0.1.0",
      schemaVersion: 1,
      startedAt: 1_700_000_000,
      checkedAt: 1_700_000_001,
    });

    renderWithClient(<HealthCheck />);

    expect(screen.getByText("Health")).toBeInTheDocument();

    await waitFor(() =>
      expect(screen.getByText(/sightline 0\.1\.0/)).toBeInTheDocument()
    );
    expect(screen.getByText("v1")).toBeInTheDocument();
  });

  test("renders an alert on error", async () => {
    vi.mocked(commands.health).mockRejectedValue(new Error("boom"));

    renderWithClient(<HealthCheck />);

    await waitFor(() => expect(screen.getByRole("alert")).toBeInTheDocument());
  });

  test("refresh triggers a refetch", async () => {
    vi.mocked(commands.health).mockResolvedValue({
      appName: "sightline",
      appVersion: "0.1.0",
      schemaVersion: 1,
      startedAt: 1_700_000_000,
      checkedAt: 1_700_000_001,
    });

    renderWithClient(<HealthCheck />);

    await waitFor(() =>
      expect(screen.getByText(/sightline 0\.1\.0/)).toBeInTheDocument()
    );

    const before = vi.mocked(commands.health).mock.calls.length;
    await userEvent.click(screen.getByRole("button", { name: /refresh/i }));
    await waitFor(() =>
      expect(vi.mocked(commands.health).mock.calls.length).toBeGreaterThan(before)
    );
  });
});
