import * as React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import FxLangModal from "./FxLangModal";
import FxLangModalProvider from "./FxLangModalProvider";
import { FxLangModalContext } from "./FxLangModalContext";
import { highlightFxlangJson } from "./fxlangHighlighter";
import * as dataStore from "../../../hooks/useDataStore";

vi.mock("../../../hooks/useDataStore", () => ({
  useGenericResource: vi.fn(),
}));

function renderWithMockedReactInternals<T>(
  fn: () => T,
  options?: {
    internals?: Record<string, unknown>;
    window?: Partial<Window & typeof globalThis>;
  },
): T {
  const originalDocument = globalThis.document;
  const originalWindow = globalThis.window;
  const internals = (
    React as unknown as {
      __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE: { H: unknown };
    }
  ).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
  const prevH = internals.H;

  try {
    internals.H = {
      useState: (init: unknown) => [init, () => {}],
      useRef: (init: unknown) => ({ current: init }),
      useCallback: (fn: unknown) => fn,
      useEffect: () => {},
      useMemo: (fn: () => unknown) => fn(),
      ...options?.internals,
    };
    if (options?.window) {
      globalThis.window = {
        ...originalWindow,
        ...options.window,
      } as unknown as Window & typeof globalThis;
    }
    globalThis.document = { body: { nodeType: 1 } } as unknown as Document;
    return fn();
  } finally {
    internals.H = prevH;
    globalThis.document = originalDocument;
    globalThis.window = originalWindow;
  }
}

interface ModalPortalStructure {
  children: {
    props: {
      className: string;
      onPointerDown?: (e: unknown) => void;
      children: {
        props: {
          onPointerDown?: (e: unknown) => void;
          children: Array<{
            props: {
              children?: any;
              [key: string]: any;
            };
          }>;
        };
      };
    };
  };
}

function getModalElements(portal: unknown) {
  const portalTyped = portal as unknown as ModalPortalStructure;
  const backdrop = portalTyped.children;
  const modal = backdrop.props.children;
  const header = modal.props.children[0];
  const content = modal.props.children[1];
  return {
    backdrop,
    modal,
    header,
    content,
    headerStr: JSON.stringify(header),
    contentStr: JSON.stringify(content),
  };
}

function createInteractiveStateTracker(initial: {
  target: { type: string; name: string; displayName?: string; tab?: string };
  history?: Array<{ type: string; name: string; displayName?: string; tab?: string }>;
  tab?: string;
}) {
  let currentTargetState: any = initial.target;
  let historyState: any[] = initial.history || [];
  let activeTabState: any = initial.tab || "fxlang";

  const useStateMock = (init: unknown) => {
    if (typeof init === "object" && init !== null && "type" in (init as any)) {
      return [
        currentTargetState,
        (updater: any) => {
          currentTargetState = typeof updater === "function" ? updater(currentTargetState) : updater;
        },
      ];
    }
    if (Array.isArray(init)) {
      return [
        historyState,
        (updater: any) => {
          historyState = typeof updater === "function" ? updater(historyState) : updater;
        },
      ];
    }
    if (init === "fxlang" || init === "effects" || init === "special") {
      return [
        activeTabState,
        (updater: any) => {
          activeTabState = typeof updater === "function" ? updater(activeTabState) : updater;
        },
      ];
    }
    return [init, () => {}];
  };

  return {
    useStateMock,
    getCurrentTarget: () => currentTargetState,
    getHistory: () => historyState,
    getActiveTab: () => activeTabState,
  };
}

describe("highlightFxlangJson", () => {
  it("highlights formatted json and applies fxlang injection tokens", async () => {
    const jsonStr = JSON.stringify(
      {
        callbacks: {
          on_source_base_power: [
            "if $move.type == func_call(value_from_local_data: type):",
            ["return $base_power * 6/5"],
          ],
        },
      },
      null,
      2,
    );

    const html = await highlightFxlangJson(jsonStr);
    expect(html).toContain('<pre class="shiki dark-plus"');
    expect(html).toContain('"callbacks"');
    expect(html).toContain('"on_source_base_power"');
    // Function name and keyword highlighted tokens
    expect(html).toContain("func_call");
    expect(html).toContain("value_from_local_data");
  });
});

describe("FxLangModal", () => {
  it("returns null when document is undefined (SSR)", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: null,
      loading: true,
    });

    const originalDoc = globalThis.document;
    // @ts-expect-error test SSR
    delete globalThis.document;

    try {
      const html = renderToStaticMarkup(
        <FxLangModal
          target={{ type: "move", name: "Thunderbolt" }}
          onClose={() => {}}
        />,
      );
      expect(html).toBe("");
    } finally {
      globalThis.document = originalDoc;
    }
  });

  it("renders modal structure with category and TypeBadge for moves", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "move",
        data: {
          name: "Thunderbolt",
          category: "Special",
          primary_type: "Electric",
          flags: [],
          hit_effect: null,
          user_effect: null,
          user_effect_chance: null,
          secondary_effects: [],
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    renderWithMockedReactInternals(() => {
      const portal = FxLangModal({
        target: { type: "move", name: "Thunderbolt" },
        onClose: () => {},
      });

      expect(portal).toBeDefined();
      const { backdrop, headerStr } = getModalElements(portal);
      expect(backdrop.props.className).toContain("backdrop");
      expect(headerStr).toContain("Thunderbolt");
      expect(headerStr).toContain("Move");
      expect(headerStr).toContain("FxLang");
      expect(headerStr).toContain("Effects");
      expect(headerStr).not.toContain('"badge"');
    });
  });

  it("renders Ability subtitle and None empty state for empty abilities", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "ability",
        data: {
          name: "Pickpocket",
          flags: [],
          effect: {},
          condition: {},
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    renderWithMockedReactInternals(() => {
      const portal = FxLangModal({
        target: { type: "ability", name: "Pickpocket" },
        onClose: () => {},
      });

      const { headerStr, contentStr } = getModalElements(portal);
      expect(headerStr).toContain("Pickpocket");
      expect(headerStr).toContain("Ability");
      expect(contentStr).toContain("None");
    });
  });

  it("renders Item subtitle and special tab for items with special_data", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "item",
        data: {
          name: "Choice Band",
          flags: [],
          special_data: {
            boosts: {
              atk: 1,
            },
          },
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    renderWithMockedReactInternals(() => {
      const portal = FxLangModal({
        target: { type: "item", name: "Choice Band" },
        onClose: () => {},
      });

      const { headerStr } = getModalElements(portal);
      expect(headerStr).toContain("Choice Band");
      expect(headerStr).toContain("Item");
      expect(headerStr).toContain("Special");
    });
  });

  it("renders Species class and stats traits for species", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "species",
        data: {
          name: "Mewtwo",
          primary_type: "Psychic",
          class: "Legendary",
          base_stats: { hp: 106, atk: 110, def: 90, spa: 154, spd: 90, spe: 130 },
          flags: [],
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    renderWithMockedReactInternals(() => {
      const portal = FxLangModal({
        target: { type: "species", name: "Mewtwo" },
        onClose: () => {},
      });

      const { headerStr } = getModalElements(portal);
      expect(headerStr).toContain("Mewtwo");
      expect(headerStr).toContain("Legendary Mon");
    });
  });

  it("stops pointerdown and keydown Escape propagation to protect background tooltips", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "move",
        data: { name: "Thunderbolt", category: "Special", primary_type: "Electric", flags: [] },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    let keydownListener: ((e: KeyboardEvent) => void) | null = null;
    let captureMode = false;
    const mockAddEventListener = vi.fn((event, fn, useCapture) => {
      if (event === "keydown") {
        keydownListener = fn as (e: KeyboardEvent) => void;
        captureMode = Boolean(useCapture);
      }
    });
    const mockRemoveEventListener = vi.fn();
    const effectFns: (() => void)[] = [];

    renderWithMockedReactInternals(
      () => {
        const onClose = vi.fn();
        const portal = FxLangModal({
          target: { type: "move", name: "Thunderbolt" },
          onClose,
        });

        const { backdrop, modal } = getModalElements(portal);

        // 1. Verify backdrop stops pointerdown
        const backdropStopPropagation = vi.fn();
        backdrop.props.onPointerDown?.({ stopPropagation: backdropStopPropagation });
        expect(backdropStopPropagation).toHaveBeenCalledTimes(1);

        // 2. Verify modal container stops pointerdown
        const modalStopPropagation = vi.fn();
        modal.props.onPointerDown?.({ stopPropagation: modalStopPropagation });
        expect(modalStopPropagation).toHaveBeenCalledTimes(1);

        // 3. Trigger effects and verify Escape key is captured with stopPropagation
        for (const fn of effectFns) fn();
        expect(keydownListener).toBeDefined();
        expect(captureMode).toBe(true);

        const escapeStopPropagation = vi.fn();
        keydownListener!({ key: "Escape", stopPropagation: escapeStopPropagation } as unknown as KeyboardEvent);
        expect(escapeStopPropagation).toHaveBeenCalledTimes(1);
        expect(onClose).toHaveBeenCalledTimes(1);
      },
      {
        internals: {
          useEffect: (fn: () => void) => {
            effectFns.push(fn);
          },
        },
        window: {
          addEventListener: mockAddEventListener,
          removeEventListener: mockRemoveEventListener,
        },
      },
    );
  });

  it("navigates to condition delegate on click and preserves previous tab on back", () => {
    const tracker = createInteractiveStateTracker({
      target: { type: "move", name: "Fake Out" },
      tab: "effects",
    });

    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "move",
        data: {
          name: "Fake Out",
          hit_effect: { volatile_status: "flinch" },
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    renderWithMockedReactInternals(
      () => {
        const portal = FxLangModal({
          target: { type: "move", name: "Fake Out" },
          onClose: () => {},
        });

        const { content } = getModalElements(portal);
        const codeViewer = content.props.children as { props: Record<string, unknown> };
        expect(codeViewer.props.onClick).toBeDefined();

        // Simulate clicking on a condition delegate span element
        const mockDelegateElement = {
          closest: (selector: string) => {
            if (selector === "[data-delegate-prefix]") {
              return {
                getAttribute: (attr: string) => {
                  if (attr === "data-delegate-prefix") return "condition";
                  if (attr === "data-delegate-name") return "flinch";
                  return null;
                },
              };
            }
            return null;
          },
        };

        const stopPropagation = vi.fn();
        const preventDefault = vi.fn();
        (codeViewer.props.onClick as any)({
          target: mockDelegateElement,
          stopPropagation,
          preventDefault,
        });

        expect(stopPropagation).toHaveBeenCalled();
        expect(preventDefault).toHaveBeenCalled();
        expect(tracker.getCurrentTarget()).toEqual({ type: "condition", name: "flinch" });
        expect(tracker.getHistory().length).toBe(1);
        expect(tracker.getHistory()[0].name).toBe("Fake Out");
        expect(tracker.getHistory()[0].tab).toBe("effects");
        expect(tracker.getActiveTab()).toBe("fxlang");
      },
      {
        internals: {
          useState: tracker.useStateMock,
        },
      },
    );
  });

  it("navigates back when clicking the back button in history", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "condition",
        data: {
          name: "Flinch",
          condition_type: "Volatile",
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    const tracker = createInteractiveStateTracker({
      target: { type: "condition", name: "flinch" },
      history: [{ type: "move", name: "Fake Out", displayName: "Fake Out", tab: "effects" }],
      tab: "fxlang",
    });

    renderWithMockedReactInternals(
      () => {
        const portal = FxLangModal({
          target: { type: "move", name: "Fake Out" },
          onClose: () => {},
        });

        const { header } = getModalElements(portal);
        const headerLeft = header.props.children[0];
        const backBtn = headerLeft.props.children[0];
        expect(backBtn.props.onClick).toBeDefined();

        backBtn.props.onClick();

        expect(tracker.getHistory().length).toBe(0);
        expect(tracker.getCurrentTarget()).toEqual({
          type: "move",
          name: "Fake Out",
          displayName: "Fake Out",
          tab: "effects",
        });
        expect(tracker.getActiveTab()).toBe("effects");
      },
      {
        internals: {
          useState: tracker.useStateMock,
        },
      },
    );
  });

  it("resolves cross-type move condition when targeted as condition (e.g. noretreat)", () => {
    let capturedOptions: any = null;
    vi.mocked(dataStore.useGenericResource).mockImplementation((_name, options) => {
      capturedOptions = options;
      return {
        data: {
          type: "move",
          data: {
            name: "No Retreat",
            category: "Status",
            primary_type: "Fighting",
            hit_effect: { volatile_status: "noretreat" },
            condition: { callbacks: { on_start: ["log_start: with_target"] } },
          },
        } as unknown as dataStore.ResourceData,
        loading: false,
      };
    });

    renderWithMockedReactInternals(() => {
      const portal = FxLangModal({
        target: { type: "condition", name: "noretreat" },
        onClose: () => {},
      });

      expect(capturedOptions?.priority).toEqual([
        "condition",
        "move",
        "ability",
        "item",
        "species",
      ]);

      const { headerStr } = getModalElements(portal);
      expect(headerStr).toContain("No Retreat");
      expect(headerStr).toContain("Move");
      expect(headerStr).toContain("FxLang");
      expect(headerStr).toContain("Effects");
    });
  });

  it("disables empty tabs with explanatory titles when a tab lacks content", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "move",
        data: {
          name: "Tackle",
          category: "Physical",
          primary_type: "Normal",
          damage: "Normal",
          hit_effect: {},
          effect: {},
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    renderWithMockedReactInternals(() => {
      const portal = FxLangModal({
        target: { type: "move", name: "Tackle" },
        onClose: () => {},
      });

      const { headerStr } = getModalElements(portal);
      expect(headerStr).toContain('"disabled":true');
      expect(headerStr).toContain("No fxlang callbacks defined");
    });
  });

  it("disables both tabs when both are empty (e.g. Horn Attack with no fxlang or effects)", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "move",
        data: {
          name: "Horn Attack",
          category: "Physical",
          primary_type: "Normal",
          hit_effect: {},
          effect: {},
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    renderWithMockedReactInternals(() => {
      const portal = FxLangModal({
        target: { type: "move", name: "Horn Attack" },
        onClose: () => {},
      });

      const { headerStr } = getModalElements(portal);
      expect(headerStr).toContain("No fxlang callbacks defined");
      expect(headerStr).toContain("No structured effects defined");
      expect(headerStr).not.toContain('"aria-selected":true');
    });
  });

  it("renders Loading... text alongside spinner when loading", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: null,
      loading: true,
    });

    renderWithMockedReactInternals(() => {
      const portal = FxLangModal({
        target: { type: "move", name: "Thunderbolt" },
        onClose: () => {},
      });

      const { contentStr } = getModalElements(portal);
      expect(contentStr).toContain("spinner");
      expect(contentStr).toContain("Loading...");
    });
  });
});

describe("FxLangModalProvider", () => {
  it("renders children cleanly and provides open/close handlers", () => {
    let contextValue: unknown = null;
    function Consumer() {
      return (
        <FxLangModalContext.Consumer>
          {(val) => {
            contextValue = val;
            return <div>Consumer Child</div>;
          }}
        </FxLangModalContext.Consumer>
      );
    }

    const html = renderToStaticMarkup(
      <FxLangModalProvider>
        <Consumer />
      </FxLangModalProvider>,
    );

    expect(html).toContain("Consumer Child");
    expect(contextValue).toBeDefined();
    expect(typeof (contextValue as Record<string, unknown>).openFxLangModal).toBe("function");
    expect(typeof (contextValue as Record<string, unknown>).closeFxLangModal).toBe("function");
  });
});
