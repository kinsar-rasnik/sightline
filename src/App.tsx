import { HealthCheck } from "@/components/HealthCheck";

export default function App() {
  return (
    <main className="min-h-screen flex items-center justify-center bg-[--color-bg] text-[--color-fg]">
      <section className="max-w-xl w-full px-8 py-12 space-y-6">
        <header className="space-y-2">
          <h1 className="text-3xl font-semibold tracking-tight">Sightline</h1>
          <p className="text-sm text-[--color-muted]">
            Multi-streamer GTA-RP VOD viewer. Phase 1 — foundation.
          </p>
        </header>
        <HealthCheck />
      </section>
    </main>
  );
}
