import type { FormattedLogDisplayItem, LogDividerType, UiNotice } from "battler-log-formatter";
import {
  formatContextValue,
  formatNoticeText as formatBaseNoticeText,
  formatUiLogEntry,
} from "battler-log-formatter";

export function formatNoticeText(notice: UiNotice): string {
  const text = formatBaseNoticeText(notice);
  const typeLower = notice.type.toLowerCase();
  if (typeLower === "damage" || typeLower === "heal") {
    return `(${text})`;
  }
  return `[${text}]`;
}

export type { FormattedLogDisplayItem, LogDividerType };
export { formatContextValue, formatUiLogEntry };


