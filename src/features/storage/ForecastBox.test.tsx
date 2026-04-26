import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { ForecastResult } from "@/ipc";

import { ForecastBox } from "./ForecastBox";

function makeForecast(overrides: Partial<ForecastResult> = {}): ForecastResult {
  return {
    weeklyDownloadGb: 12.3,
    peakDiskGb: 4.5,
    watermarkRisk: "green",
    avgVodHours: 3.0,
    streamsPerDay: 0.5,
    freeDiskGb: 200.0,
    dataDriven: true,
    ...overrides,
  };
}

describe("ForecastBox", () => {
  it("renders the weekly + peak numbers with one decimal", () => {
    render(<ForecastBox forecast={makeForecast()} />);
    expect(screen.getByText(/12\.3 GB \/ week/)).toBeInTheDocument();
    expect(screen.getByText(/4\.5 GB/)).toBeInTheDocument();
  });

  it("shows 'Within budget' badge for green risk", () => {
    render(<ForecastBox forecast={makeForecast({ watermarkRisk: "green" })} />);
    const badge = screen.getByRole("status", { name: /Watermark risk/ });
    expect(within(badge).getByText("Within budget")).toBeInTheDocument();
  });

  it("shows 'Close to limit' for amber risk", () => {
    render(<ForecastBox forecast={makeForecast({ watermarkRisk: "amber" })} />);
    expect(screen.getByText("Close to limit")).toBeInTheDocument();
  });

  it("shows 'Over limit' alert + warning copy for red risk", () => {
    render(<ForecastBox forecast={makeForecast({ watermarkRisk: "red" })} />);
    expect(screen.getByText("Over limit")).toBeInTheDocument();
    expect(
      screen.getByRole("alert").textContent
    ).toMatch(/auto-cleanup high watermark/);
  });

  it("surfaces the fallback note when data is not data-driven", () => {
    render(<ForecastBox forecast={makeForecast({ dataDriven: false })} />);
    expect(
      screen.getByRole("note").textContent
    ).toMatch(/global defaults/i);
  });

  it("does not surface the fallback note when data-driven", () => {
    render(<ForecastBox forecast={makeForecast({ dataDriven: true })} />);
    expect(screen.queryByRole("note")).toBeNull();
  });

  it("surfaces 'Library not configured' when freeDiskGb is 0", () => {
    render(<ForecastBox forecast={makeForecast({ freeDiskGb: 0 })} />);
    expect(screen.getByText(/Library not configured/)).toBeInTheDocument();
  });

  it("uses the provided title", () => {
    render(<ForecastBox forecast={makeForecast()} title="My custom title" />);
    expect(
      screen.getByRole("heading", { name: "My custom title" })
    ).toBeInTheDocument();
  });
});
