import { useEffect, useRef, type ReactNode } from "react";

/**
 * Slide-in drawer from the right edge. Uses a minimal focus-trap so
 * keyboard users can't escape into the (visually covered) main page
 * while the drawer is open; Escape closes it.
 */
export function Drawer({
  open,
  onClose,
  title,
  children,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
      if (e.key === "Tab" && ref.current) {
        // Simple focus trap: keep Tab inside the drawer.
        const focusable = ref.current.querySelectorAll<HTMLElement>(
          'a[href], button, [tabindex]:not([tabindex="-1"]), input, select, textarea'
        );
        if (focusable.length === 0) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (!first || !last) return;
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      previous?.focus();
    };
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div
      className="fixed inset-0 z-40"
      role="dialog"
      aria-modal="true"
      aria-label={title}
    >
      <button
        type="button"
        aria-label="Close drawer"
        onClick={onClose}
        className="absolute inset-0 bg-black/40 sightline-fade-in"
      />
      <div
        ref={ref}
        tabIndex={-1}
        className="absolute right-0 top-0 bottom-0 w-full max-w-md bg-[--color-surface-elevated] border-l border-[--color-border] shadow-[var(--shadow-lg)] sightline-slide-in overflow-y-auto"
      >
        <div className="sticky top-0 bg-[--color-surface-elevated] border-b border-[--color-border] px-5 py-3 flex items-center justify-between">
          <h3 className="text-sm font-semibold">{title}</h3>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="text-[--color-muted] hover:text-[--color-fg] px-2 py-1"
          >
            ×
          </button>
        </div>
        <div className="p-5">{children}</div>
      </div>
    </div>
  );
}
