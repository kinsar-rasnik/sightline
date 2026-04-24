import { IpcError } from "@/ipc";

export function ErrorBanner({ error }: { error: unknown }) {
  if (!error) return null;
  let text: string;
  if (error instanceof IpcError) {
    const detail =
      "detail" in error.appError ? error.appError.detail : error.appError.kind;
    text = `${error.appError.kind}: ${detail}`;
  } else if (error instanceof Error) {
    text = error.message;
  } else {
    text = String(error);
  }
  return (
    <p
      role="alert"
      className="text-sm text-red-400 bg-red-500/10 border border-red-500/40 rounded px-3 py-2"
    >
      {text}
    </p>
  );
}
