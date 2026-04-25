import { describe, expect, it } from "vitest";

import {
  RESTART_THRESHOLD_SECONDS,
  clampSpeed,
  computeInitialSeekSeconds,
  speedStepDown,
  speedStepUp,
} from "./player-constants";

describe("computeInitialSeekSeconds", () => {
  const baseInput = {
    durationSeconds: 3600,
    preRollSeconds: 5,
    restartThresholdSeconds: RESTART_THRESHOLD_SECONDS,
  };

  it("returns 0 for non-finite or zero duration", () => {
    expect(
      computeInitialSeekSeconds({ ...baseInput, durationSeconds: 0 })
    ).toBe(0);
    expect(
      computeInitialSeekSeconds({ ...baseInput, durationSeconds: NaN })
    ).toBe(0);
  });

  it("explicit initialPositionSeconds overrides stored progress", () => {
    expect(
      computeInitialSeekSeconds({
        ...baseInput,
        storedPositionSeconds: 1000,
        initialPositionSeconds: 250,
      })
    ).toBe(250);
  });

  it("explicit deep-link is clamped into [0, duration]", () => {
    expect(
      computeInitialSeekSeconds({
        ...baseInput,
        initialPositionSeconds: -50,
      })
    ).toBe(0);
    expect(
      computeInitialSeekSeconds({
        ...baseInput,
        initialPositionSeconds: 99_999,
      })
    ).toBe(3600);
  });

  it("subtracts pre-roll from stored position when not at end", () => {
    expect(
      computeInitialSeekSeconds({
        ...baseInput,
        storedPositionSeconds: 100,
      })
    ).toBe(95);
  });

  it("clamps pre-roll subtraction at 0", () => {
    expect(
      computeInitialSeekSeconds({
        ...baseInput,
        storedPositionSeconds: 3,
      })
    ).toBe(0);
  });

  it("returns 0 when stored is within restart threshold of the end", () => {
    expect(
      computeInitialSeekSeconds({
        ...baseInput,
        storedPositionSeconds: 3590, // last 10 s of a 3600 s VOD
      })
    ).toBe(0);
    expect(
      computeInitialSeekSeconds({
        ...baseInput,
        storedPositionSeconds: 3570, // exactly on the boundary
      })
    ).toBe(0);
  });

  it("returns 0 when stored position is missing or zero", () => {
    expect(computeInitialSeekSeconds(baseInput)).toBe(0);
    expect(
      computeInitialSeekSeconds({ ...baseInput, storedPositionSeconds: 0 })
    ).toBe(0);
  });

  it("respects custom pre-roll values", () => {
    expect(
      computeInitialSeekSeconds({
        ...baseInput,
        preRollSeconds: 30,
        storedPositionSeconds: 500,
      })
    ).toBe(470);
  });
});

describe("speed step helpers", () => {
  it("speedStepUp moves forward through the supported steps", () => {
    expect(speedStepUp(1)).toBe(1.25);
    expect(speedStepUp(1.75)).toBe(2);
    expect(speedStepUp(2)).toBe(2); // saturate at top
  });

  it("speedStepDown moves backward through the supported steps", () => {
    expect(speedStepDown(1)).toBe(0.75);
    expect(speedStepDown(0.5)).toBe(0.5); // saturate at bottom
  });

  it("clampSpeed snaps to the nearest supported step", () => {
    expect(clampSpeed(0.6)).toBe(0.5);
    expect(clampSpeed(0.9)).toBe(1);
    expect(clampSpeed(1.4)).toBe(1.5);
  });
});
