import type { BattleMetadata, BattlePreview } from "battler-service-client";
import type { BattleState, UiLogEntry } from "battler-state";
import type { PlayerBattleData, Request } from "battler-types";
import type { RootState } from "../store/store";
import { store } from "../store/store";

export interface BugReportEnvironment {
  userAgent: string;
  viewport: string;
}

export interface BugReportReactCrash {
  message: string;
  stack?: string;
  componentStack?: string;
}

export interface BugReportBattleDebug {
  battleId: string;
  metadata?: BattleMetadata;
  battleState: BattleState | null;
  activeRequest: Request | null;
  playerData: PlayerBattleData | null;
  uiLogs: UiLogEntry[];
  engineLogs: string[];
  error: string | null;
  choiceError: string | null;
  preview?: BattlePreview | null;
}

export interface BugReportPayload {
  title: string;
  description: string;
  view: string;
  environment: BugReportEnvironment;
  reactCrash?: BugReportReactCrash;
  battleDebug?: BugReportBattleDebug;
}

export interface BugReportResponse {
  success: boolean;
  issueUrl: string;
  issueNumber: number;
  reportId: string;
  dryRun?: boolean;
  message?: string;
  softLaunch?: boolean;
}

/**
 * Gathers the complete diagnostic snapshot for a bug report using the current
 * browser environment and Redux application state.
 */
export function gatherBugReportPayload(
  title: string,
  description: string,
  reactCrash?: BugReportReactCrash,
  injectedState?: RootState,
): BugReportPayload {
  const state: RootState = injectedState ?? store.getState();

  const userAgent = typeof navigator !== "undefined" ? navigator.userAgent : "unknown";
  const viewport =
    typeof window !== "undefined" ? `${window.innerWidth}x${window.innerHeight}` : "unknown";

  const payload: BugReportPayload = {
    title,
    description,
    view: state.battles.currentView,
    environment: {
      userAgent,
      viewport,
    },
  };

  if (reactCrash) {
    payload.reactCrash = reactCrash;
  }

  const activeBattleId = state.battles.activeBattleId;
  const battleSession = activeBattleId ? state.battles.battles[activeBattleId] : undefined;

  if (battleSession && activeBattleId) {
    payload.battleDebug = {
      battleId: activeBattleId,
      metadata: battleSession.metadata ?? battleSession.serviceBattle?.metadata,
      battleState: battleSession.battleState,
      activeRequest: battleSession.activeRequest,
      playerData: battleSession.playerData,
      uiLogs: battleSession.uiLogs,
      engineLogs: battleSession.isReplay
        ? battleSession.replayEngineLogs
        : battleSession.engineLogs,
      error: battleSession.error,
      choiceError: battleSession.choiceError,
      preview: battleSession.preview,
    };
  }

  return payload;
}

/**
 * Default URL for the bug reporting backend relay service.
 */
export const DEFAULT_BUG_REPORT_API_URL =
  import.meta.env?.VITE_BUG_REPORT_API_URL || "http://localhost:5000/api/report-bug";

/**
 * Submits the diagnostic payload to the backend relay service.
 */
export async function submitBugReport(
  payload: BugReportPayload,
  apiUrl: string = DEFAULT_BUG_REPORT_API_URL,
): Promise<BugReportResponse> {
  const response = await fetch(apiUrl, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    let errorMsg = `Server returned HTTP ${response.status}`;
    try {
      const errorJson = await response.json();
      if (errorJson?.error) {
        errorMsg = errorJson.error;
      } else if (errorJson?.detail) {
        errorMsg = errorJson.detail;
      }
    } catch {
      // Use fallback status text
    }
    throw new Error(errorMsg);
  }

  return response.json();
}

/**
 * Generates a pre-filled GitHub issue creation URL for fallback reporting.
 */
export function getGitHubFallbackUrl(payload: BugReportPayload): string {
  const repoOwner = "jackson-nestelroad";
  const repoName = "battler";

  const battleIdInfo = payload.battleDebug?.battleId
    ? `* **Battle ID**: \`${payload.battleDebug.battleId}\`\n`
    : "";

  const crashInfo = payload.reactCrash
    ? `\n<details>\n<summary><b>Crash Error</b></summary>\n\n\`\`\`text\n${payload.reactCrash.message}\n${payload.reactCrash.stack || ""}\n\`\`\`\n</details>\n`
    : "";

  const body = `### Summary
${payload.description}

### Environment & Context
* **View**: \`${payload.view}\`
* **Browser**: \`${payload.environment.userAgent}\`
* **Viewport**: \`${payload.environment.viewport}\`
${battleIdInfo}${crashInfo}
> [!NOTE]
> Please attach the downloaded diagnostic JSON file to this issue for full investigation.`;

  return `https://github.com/${repoOwner}/${repoName}/issues/new?title=${encodeURIComponent(
    `[Bug]: ${payload.title}`,
  )}&body=${encodeURIComponent(body)}&labels=bug,web-app`;
}

/**
 * Triggers a client-side download of the diagnostic payload as a JSON file.
 */
export function downloadDiagnosticJson(payload: BugReportPayload): void {
  const filename = `battler-debug-${Date.now()}.json`;
  const jsonStr = JSON.stringify(payload, null, 2);
  const blob = new Blob([jsonStr], { type: "application/json" });
  const url =
    typeof URL.createObjectURL === "function"
      ? URL.createObjectURL(blob)
      : `data:application/json;charset=utf-8,${encodeURIComponent(jsonStr)}`;

  const anchor = document.createElement("a");
  anchor.setAttribute("href", url);
  anchor.setAttribute("download", filename);
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();

  if (typeof URL.revokeObjectURL === "function" && url.startsWith("blob:")) {
    URL.revokeObjectURL(url);
  }
}
