import type { BattleState, UiLogEntry } from "battler-state";
import type { FormattedUiLog, LogCategory, UiNotice } from "battler-log-formatter";
import { LogFormatter } from "battler-log-formatter";

export type FormattedLogDisplayItem =
  | {
      kind: "turn";
      turn: string;
    }
  | {
      kind: "request-split";
    }
  | {
      kind: "message";
      category: LogCategory;
      message: FormattedUiLog;
    }
  | {
      kind: "notice";
      notice: UiNotice;
    };

function getFormatter(localPlayerId?: string): LogFormatter {
  // Try to use a singleton formatter, or re-instantiate if player ID changes
  // For now, just create one on the fly since we need it rarely
  return new LogFormatter({ localPlayerId });
}

export function formatNoticeText(notice: UiNotice): string {
  switch (notice.type.toLowerCase()) {
    case "ability":
    case "item": {
      const subject = notice.mon ? `${notice.mon} ` : "";
      return `[${subject}${notice.name}]`;
    }
    case "damage": {
      const subject = notice.mon ? `${notice.mon} ` : "";
      return `(${subject}lost ${notice.name} HP)`;
    }
    case "heal": {
      const subject = notice.mon ? `${notice.mon} ` : "";
      return `(${subject}restored ${notice.name} HP)`;
    }
    default: {
      const subject = notice.mon ? `${notice.mon} ` : "";
      return `[${notice.type}: ${subject}${notice.name}]`;
    }
  }
}

export function formatUiLogEntry(
  entry: UiLogEntry,
  state?: BattleState,
  localPlayerId?: string,
): FormattedLogDisplayItem[] {
  const titleLower = entry.title.toLowerCase();
  if (titleLower === "turn") {
    const turnVal = entry.values?.turn;
    const turnStr = turnVal !== undefined ? String(turnVal) : "";
    return [{ kind: "turn", turn: turnStr }];
  }

  if (titleLower === "continue" || titleLower === "time") {
    return [{ kind: "request-split" }];
  }

  const formatter = getFormatter(localPlayerId);
  const event = formatter.format(entry, state);
  if (!event) return [];


  const preNotices: FormattedLogDisplayItem[] = [];
  const postNotices: FormattedLogDisplayItem[] = [];

  for (const notice of event.notices) {
    const typeLower = notice.type.toLowerCase();
    if (typeLower === "damage" || typeLower === "heal") {
      postNotices.push({ kind: "notice", notice });
    } else {
      preNotices.push({ kind: "notice", notice });
    }
  }

  const messages: FormattedLogDisplayItem[] = event.messages.map((msg) => ({
    kind: "message",
    category: msg.category,
    message: msg,
  }));

  return [...preNotices, ...messages, ...postNotices];
}

