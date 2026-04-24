import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "secondary" | "danger";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  children: ReactNode;
}

const variantClasses: Record<Variant, string> = {
  primary:
    "bg-[--color-accent] text-white hover:opacity-90 border border-transparent",
  secondary:
    "bg-transparent text-[--color-fg] border border-[--color-border] hover:bg-[--color-surface]",
  danger:
    "bg-transparent text-red-400 border border-red-500/40 hover:bg-red-500/10",
};

export function Button({
  variant = "secondary",
  className = "",
  children,
  ...rest
}: Props) {
  return (
    <button
      {...rest}
      className={`text-sm px-3 py-1.5 rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent] transition-opacity disabled:opacity-50 disabled:cursor-not-allowed ${variantClasses[variant]} ${className}`}
    >
      {children}
    </button>
  );
}
