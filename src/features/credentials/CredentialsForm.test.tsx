import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, test, vi } from "vitest";

import { CredentialsForm } from "@/features/credentials/CredentialsForm";
import type { CredentialsStatus } from "@/ipc";

import type * as IpcModule from "@/ipc";

vi.mock("@/ipc", async () => {
  const actual = await vi.importActual<typeof IpcModule>("@/ipc");
  return {
    ...actual,
    commands: {
      ...actual.commands,
      setTwitchCredentials: vi.fn(),
      clearTwitchCredentials: vi.fn(),
      getTwitchCredentialsStatus: vi.fn(),
    },
  };
});

import { commands } from "@/ipc";

function renderWith(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe("CredentialsForm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test("renders the unconfigured form with inputs", () => {
    const status: CredentialsStatus = {
      configured: false,
      client_id_masked: null,
      last_token_acquired_at: null,
    };
    renderWith(<CredentialsForm status={status} />);
    expect(screen.getByLabelText(/client id/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/client secret/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /save credentials/i })
    ).toBeInTheDocument();
  });

  test("rejects empty submission with a local validation error", async () => {
    const status: CredentialsStatus = {
      configured: false,
      client_id_masked: null,
      last_token_acquired_at: null,
    };
    renderWith(<CredentialsForm status={status} />);
    await userEvent.click(
      screen.getByRole("button", { name: /save credentials/i })
    );
    expect(
      await screen.findByText(/client id and client secret are both required/i)
    ).toBeInTheDocument();
    expect(vi.mocked(commands.setTwitchCredentials)).not.toHaveBeenCalled();
  });

  test("submits trimmed values and calls the IPC", async () => {
    const status: CredentialsStatus = {
      configured: false,
      client_id_masked: null,
      last_token_acquired_at: null,
    };
    vi.mocked(commands.setTwitchCredentials).mockResolvedValue({
      configured: true,
      client_id_masked: "abcd••••",
      last_token_acquired_at: 1_700_000_000,
    });
    renderWith(<CredentialsForm status={status} />);
    await userEvent.type(screen.getByLabelText(/client id/i), "abcdWXYZ");
    await userEvent.type(screen.getByLabelText(/client secret/i), "secret");
    await userEvent.click(
      screen.getByRole("button", { name: /save credentials/i })
    );
    await waitFor(() =>
      expect(vi.mocked(commands.setTwitchCredentials)).toHaveBeenCalledWith({
        clientId: "abcdWXYZ",
        clientSecret: "secret",
      })
    );
  });

  test("renders the masked Client ID when configured and supports Replace", async () => {
    const status: CredentialsStatus = {
      configured: true,
      client_id_masked: "abcd••••",
      last_token_acquired_at: 1_700_000_000,
    };
    vi.mocked(commands.clearTwitchCredentials).mockResolvedValue(undefined);
    renderWith(<CredentialsForm status={status} />);
    expect(screen.getByText(/configured/i)).toBeInTheDocument();
    expect(screen.getByText(/abcd••••/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /replace/i }));
    await waitFor(() =>
      expect(vi.mocked(commands.clearTwitchCredentials)).toHaveBeenCalled()
    );
  });
});
