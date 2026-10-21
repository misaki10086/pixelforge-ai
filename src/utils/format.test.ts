import { describe, expect, it } from "vitest";
import { formatDate, formatDuration, formatRelative, truncate } from "./format";

describe("formatDuration", () => {
  it("formats milliseconds under one second", () => {
    expect(formatDuration(400)).toBe("400ms");
  });
  it("formats seconds", () => {
    expect(formatDuration(1500)).toBe("1.5s");
  });
  it("formats minutes", () => {
    expect(formatDuration(125000)).toBe("2m 5s");
  });
  it("handles null", () => {
    expect(formatDuration(null)).toBe("—");
  });
});

describe("truncate", () => {
  it("keeps short text", () => {
    expect(truncate("hello", 10)).toBe("hello");
  });
  it("cuts long text with ellipsis", () => {
    expect(truncate("hello world", 8)).toBe("hello w…");
  });
});

describe("formatDate / formatRelative", () => {
  it("returns dash for null", () => {
    expect(formatDate(null)).toBe("—");
    expect(formatRelative(null)).toBe("从未");
  });
  it("formats a valid date", () => {
    expect(formatDate("2026-08-31T10:00:00Z")).toMatch(/^2026-/);
  });
  it("formats just-now relative time", () => {
    expect(formatRelative(new Date().toISOString())).toBe("刚刚");
  });
  it("returns dash for invalid input", () => {
    expect(formatDate("not-a-date")).toBe("—");
  });
});
