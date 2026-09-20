import { describe, expect, it, vi } from "vitest";
import type { RootState } from "../store/store";
import {
  gatherBugReportPayload,
  getGitHubFallbackUrl,
  submitBugReport,
} from "./bugReport";

describe("bugReport core utilities", () => {
  it("gathers base diagnostics without active battle", () => {
    const mockState = {
      battles: {
        activeBattleId: null,
        currentView: "lobby",
        battles: {},
      },
      connection: {
        status: "connected",
      },
    } as unknown as RootState;

    const payload = gatherBugReportPayload(
      "Test Title",
      "Test Description",
      undefined,
      mockState,
    );

    expect(payload.title).toBe("Test Title");
    expect(payload.description).toBe("Test Description");
    expect(payload.view).toBe("lobby");
    expect(payload.battleDebug).toBeUndefined();
    expect(payload.environment.userAgent).toBeDefined();
    expect(payload.environment.viewport).toBeDefined();
  });

  it("gathers battle diagnostics when active battle exists", () => {
    const mockBattleId = "battle-1234";
    const mockState = {
      battles: {
        activeBattleId: mockBattleId,
        currentView: "battle",
        battles: {
          [mockBattleId]: {
            battleId: mockBattleId,
            battleState: { phase: "turn", turn: 5 },
            activeRequest: null,
            playerData: null,
            uiLogs: [{ message: "Pikachu used Thunderbolt" }],
            engineLogs: ["turn|turn:5", "move|mon:p1:Pikachu"],
            error: null,
            choiceError: null,
          },
        },
      },
    } as unknown as RootState;

    const payload = gatherBugReportPayload(
      "Battle Glitch",
      "Move failed",
      undefined,
      mockState,
    );

    expect(payload.view).toBe("battle");
    expect(payload.battleDebug).toBeDefined();
    expect(payload.battleDebug?.battleId).toBe(mockBattleId);
    expect(payload.battleDebug?.engineLogs).toHaveLength(2);
    expect(payload.battleDebug?.uiLogs).toHaveLength(1);
  });

  it("includes reactCrash details when provided", () => {
    const mockState = {
      battles: { activeBattleId: null, currentView: "teams", battles: {} },
    } as unknown as RootState;

    const crash = {
      message: "Uncaught TypeError: Cannot read null",
      stack: "Error: at Component.render()",
      componentStack: "in MonCard\n in TeamEditor",
    };

    const payload = gatherBugReportPayload("Crash", "App broke", crash, mockState);

    expect(payload.view).toBe("teams");
    expect(payload.reactCrash).toEqual(crash);
  });

  it("generates correct fallback GitHub Issue URL", () => {
    const payload = {
      title: "Broken UI",
      description: "Button did not click",
      view: "teams",
      environment: {
        userAgent: "TestBrowser",
        viewport: "1920x1080",
      },
    };

    const url = getGitHubFallbackUrl(payload);
    expect(url).toContain("https://github.com/jackson-nestelroad/battler/issues/new");
    expect(url).toContain("Broken%20UI");
    expect(url).toContain("Button%20did%20not%20click");
    expect(url).toContain("labels=bug,web-app");
  });

  it("submits bug report successfully via fetch", async () => {
    const mockResponse = {
      success: true,
      issueUrl: "https://github.com/jackson-nestelroad/battler/issues/42",
      issueNumber: 42,
      reportId: "uuid-1234",
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockResponse,
    });

    const payload = {
      title: "Test",
      description: "Test description",
      view: "lobby",
      environment: { userAgent: "test", viewport: "1000x800" },
    };

    const result = await submitBugReport(payload, "http://test-relay/api/report-bug");
    expect(result).toEqual(mockResponse);
    expect(global.fetch).toHaveBeenCalledWith(
      "http://test-relay/api/report-bug",
      expect.objectContaining({
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      }),
    );
  });

  it("handles fetch failure with server error message", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({ error: "Title and description are required" }),
    });

    const payload = {
      title: "",
      description: "",
      view: "lobby",
      environment: { userAgent: "test", viewport: "1000x800" },
    };

    await expect(
      submitBugReport(payload, "http://test-relay/api/report-bug"),
    ).rejects.toThrow("Title and description are required");
  });
});
