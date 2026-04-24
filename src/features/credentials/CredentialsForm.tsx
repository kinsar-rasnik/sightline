import { useState } from "react";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import type { CredentialsStatus } from "@/ipc";
import { formatUnixSeconds } from "@/lib/format";
import {
  useClearCredentials,
  useSetCredentials,
} from "@/features/settings/use-settings";

interface Props {
  status: CredentialsStatus;
}

export function CredentialsForm({ status }: Props) {
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  const setMut = useSetCredentials();
  const clearMut = useClearCredentials();

  const submit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setFormError(null);
    if (!clientId.trim() || !clientSecret.trim()) {
      setFormError("Client ID and Client Secret are both required.");
      return;
    }
    try {
      await setMut.mutateAsync({ clientId, clientSecret });
      setClientId("");
      setClientSecret("");
    } catch {
      // error surfaces via setMut.error below.
    }
  };

  if (status.configured) {
    return (
      <section aria-labelledby="credentials-heading" className="space-y-3">
        <div className="flex items-baseline justify-between gap-4">
          <h3 id="credentials-heading" className="text-base font-medium">
            Twitch credentials
          </h3>
          <span className="text-xs text-[--color-muted]">
            Configured · Client ID {status.client_id_masked ?? "••••"}
          </span>
        </div>
        <p className="text-sm text-[--color-muted]">
          Last token acquired: {formatUnixSeconds(status.last_token_acquired_at)}
        </p>
        <div className="flex gap-2">
          <Button
            variant="danger"
            onClick={() => clearMut.mutate()}
            disabled={clearMut.isPending}
          >
            {clearMut.isPending ? "Clearing…" : "Replace"}
          </Button>
        </div>
        <ErrorBanner error={clearMut.error} />
      </section>
    );
  }

  return (
    <section aria-labelledby="credentials-heading" className="space-y-3">
      <h3 id="credentials-heading" className="text-base font-medium">
        Twitch credentials
      </h3>
      <p className="text-xs text-[--color-muted] max-w-prose">
        Create a dev application at{" "}
        <span className="font-mono">dev.twitch.tv/console/apps</span> and paste
        the Client ID and Client Secret below. Sightline stores them in your
        OS keychain and never writes them to disk in plaintext.
      </p>
      <form onSubmit={submit} className="space-y-3 max-w-md">
        <label className="block text-sm">
          <span className="text-[--color-muted]">Client ID</span>
          <input
            type="text"
            autoComplete="off"
            value={clientId}
            onChange={(e) => setClientId(e.target.value)}
            className="mt-1 w-full rounded border border-[--color-border] bg-[--color-surface] px-3 py-2 text-sm font-mono focus:outline focus:outline-2 focus:outline-[--color-accent]"
          />
        </label>
        <label className="block text-sm">
          <span className="text-[--color-muted]">Client Secret</span>
          <input
            type="password"
            autoComplete="off"
            value={clientSecret}
            onChange={(e) => setClientSecret(e.target.value)}
            className="mt-1 w-full rounded border border-[--color-border] bg-[--color-surface] px-3 py-2 text-sm font-mono focus:outline focus:outline-2 focus:outline-[--color-accent]"
          />
        </label>
        {formError && (
          <p role="alert" className="text-sm text-red-400">
            {formError}
          </p>
        )}
        <ErrorBanner error={setMut.error} />
        <Button type="submit" variant="primary" disabled={setMut.isPending}>
          {setMut.isPending ? "Saving…" : "Save credentials"}
        </Button>
      </form>
    </section>
  );
}
