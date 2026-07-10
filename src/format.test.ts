import { describe, expect, it } from "vitest";
import {
  displayPercent,
  formatResetTime,
  remainingPercent,
  usageTone,
} from "./format";

describe("usage formatting", () => {
  it("rounds a percentage for compact display", () => {
    expect(displayPercent(31.6)).toBe("32%");
    expect(displayPercent(null)).toBe("—");
  });

  it("formats reset durations without depending on the local timezone", () => {
    expect(formatResetTime(1_000 + 30 * 60, 1_000)).toBe("Resets in 30m");
    expect(formatResetTime(1_000 + 5 * 60 * 60, 1_000)).toBe("Resets in 5h");
    expect(formatResetTime(1_000 + 50 * 60 * 60, 1_000)).toBe("Resets in 2d 2h");
    expect(formatResetTime(1_000 + 5 * 60 * 60, 1_000, "zh")).toBe(
      "5 小时后重置",
    );
  });

  it("turns used percentage into remaining percentage", () => {
    expect(remainingPercent(44)).toBe(56);
    expect(remainingPercent(null)).toBeNull();
  });

  it("changes tone only when little quota remains", () => {
    expect(usageTone(31)).toBe("calm");
    expect(usageTone(30)).toBe("watch");
    expect(usageTone(10)).toBe("limit");
  });
});
