import type { BattleState, UiLogEntry } from "battler-state";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { UseFormattedLogsOptions } from "./useFormattedLogs";
import { useFormattedLogs } from "./useFormattedLogs";

function TestConsumer(props: UseFormattedLogsOptions) {
  const logs = useFormattedLogs(props);
  return (
    <div data-count={logs.length} id="output">
      {logs.map((item, idx) => (
        <span key={idx} data-kind={item.kind}>
          {item.kind === "turn" ? `Turn ${item.turn}` : ""}
        </span>
      ))}
    </div>
  );
}

describe("useFormattedLogs", () => {
  it("returns empty array when state or logs are empty", () => {
    const html = renderToStaticMarkup(<TestConsumer uiLogs={[]} battleState={null} />);
    expect(html).toContain('data-count="0"');
  });

  it("formats logs when state and uiLogs are provided", () => {
    const mockState = { turn: 1 } as unknown as BattleState;
    const mockLogs: UiLogEntry[] = [
      { title: "turn", values: { turn: 1 } } as unknown as UiLogEntry,
    ];
    const html = renderToStaticMarkup(
      <TestConsumer uiLogs={mockLogs} battleState={mockState} battleId="test-battle" />,
    );
    expect(html).toContain('data-count="1"');
    expect(html).toContain("Turn 1");
  });

  it("formats newly appended logs incrementally", () => {
    const mockState = { turn: 2 } as unknown as BattleState;
    const logs: UiLogEntry[] = [
      { title: "turn", values: { turn: 1 } } as unknown as UiLogEntry,
      { title: "turn", values: { turn: 2 } } as unknown as UiLogEntry,
    ];
    const html = renderToStaticMarkup(
      <TestConsumer uiLogs={logs} battleState={mockState} battleId="test-battle" />,
    );
    expect(html).toContain('data-count="2"');
    expect(html).toContain("Turn 1");
    expect(html).toContain("Turn 2");
  });

  it("resets cache when battleId changes", () => {
    const mockState = { turn: 1 } as unknown as BattleState;
    const logs1: UiLogEntry[] = [
      { title: "turn", values: { turn: 1 } } as unknown as UiLogEntry,
    ];
    const logs2: UiLogEntry[] = [
      { title: "turn", values: { turn: 5 } } as unknown as UiLogEntry,
    ];
    const html1 = renderToStaticMarkup(
      <TestConsumer uiLogs={logs1} battleState={mockState} battleId="battle-1" />,
    );
    expect(html1).toContain("Turn 1");

    const html2 = renderToStaticMarkup(
      <TestConsumer uiLogs={logs2} battleState={mockState} battleId="battle-2" />,
    );
    expect(html2).toContain("Turn 5");
    expect(html2).not.toContain("Turn 1");
  });
});
