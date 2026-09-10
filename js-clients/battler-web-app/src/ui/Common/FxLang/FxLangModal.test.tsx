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

    const originalDocument = globalThis.document;
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
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FxLangModal({
        target: { type: "move", name: "Thunderbolt" },
        onClose: () => {},
      });

      expect(portal).toBeDefined();
      const portalAny = portal as unknown as {
        children: {
          props: {
            className: string;
            children: {
              props: {
                children: Array<{
                  props: Record<string, unknown>;
                }>;
              };
            };
          };
        };
      };

      const backdrop = portalAny.children;
      expect(backdrop.props.className).toContain("backdrop");

      const header = backdrop.props.children.props.children[0];
      const headerStr = JSON.stringify(header);
      expect(headerStr).toContain("Thunderbolt");
      expect(headerStr).toContain("Move");
      expect(headerStr).toContain("fxlang");
      expect(headerStr).toContain("effects");
      // No awkward lowercase move chip
      expect(headerStr).not.toContain('"badge"');
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
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

    const originalDocument = globalThis.document;
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
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FxLangModal({
        target: { type: "ability", name: "Pickpocket" },
        onClose: () => {},
      });

      const portalAny = portal as unknown as {
        children: {
          props: {
            children: {
              props: {
                children: Array<{
                  props: Record<string, unknown>;
                }>;
              };
            };
          };
        };
      };

      const header = portalAny.children.props.children.props.children[0];
      const headerStr = JSON.stringify(header);
      expect(headerStr).toContain("Pickpocket");
      expect(headerStr).toContain("Ability");
      // Move-specific tabbar should be omitted
      expect(headerStr).not.toContain("Move effect views");

      // Verify empty state is "None"
      const body = portalAny.children.props.children.props.children[1];
      const bodyStr = JSON.stringify(body);
      expect(bodyStr).toContain("None");
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
  });

  it("renders Item tabs (fxlang, special_data) for items", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "item",
        data: {
          name: "Normalium Z",
          flags: [],
          effect: {},
          special_data: {
            z_crystal: { type: "Normal", into: "Breakneck Blitz" },
            judgment: { type: "Normal" },
          },
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    const originalDocument = globalThis.document;
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
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FxLangModal({
        target: { type: "item", name: "Normalium Z" },
        onClose: () => {},
      });

      const portalAny = portal as unknown as {
        children: {
          props: {
            children: {
              props: {
                children: Array<{
                  props: Record<string, unknown>;
                }>;
              };
            };
          };
        };
      };

      const header = portalAny.children.props.children.props.children[0];
      const headerStr = JSON.stringify(header);
      expect(headerStr).toContain("Normalium Z");
      expect(headerStr).toContain("Item");
      expect(headerStr).toContain("fxlang");
      expect(headerStr).toContain("special");
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
  });

  it("renders Species class subtitle and handles species fxlang", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "species",
        data: {
          name: "Xerneas",
          class: "Life",
          effect: {
            callbacks: {
              on_switch_in: ["run_event_on_mon_species: Update"],
            },
          },
        },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    const originalDocument = globalThis.document;
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
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FxLangModal({
        target: { type: "species", name: "Xerneas" },
        onClose: () => {},
      });

      const portalAny = portal as unknown as {
        children: {
          props: {
            children: {
              props: {
                children: Array<{
                  props: Record<string, unknown>;
                }>;
              };
            };
          };
        };
      };

      const header = portalAny.children.props.children.props.children[0];
      const headerStr = JSON.stringify(header);
      expect(headerStr).toContain("Xerneas");
      expect(headerStr).toContain("Life Mon");
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
  });

  it("stops pointerdown and keydown Escape propagation to protect background tooltips", () => {
    vi.mocked(dataStore.useGenericResource).mockReturnValue({
      data: {
        type: "move",
        data: { name: "Thunderbolt", category: "Special", primary_type: "Electric", flags: [] },
      } as unknown as dataStore.ResourceData,
      loading: false,
    });

    const originalDocument = globalThis.document;
    const originalWindow = globalThis.window;
    const internals = (
      React as unknown as {
        __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE: { H: unknown };
      }
    ).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
    const prevH = internals.H;

    let keydownListener: ((e: KeyboardEvent) => void) | null = null;
    let captureMode = false;
    const mockAddEventListener = vi.fn((event, fn, useCapture) => {
      if (event === "keydown") {
        keydownListener = fn as (e: KeyboardEvent) => void;
        captureMode = Boolean(useCapture);
      }
    });
    const mockRemoveEventListener = vi.fn();
    globalThis.window = {
      addEventListener: mockAddEventListener,
      removeEventListener: mockRemoveEventListener,
    } as unknown as Window & typeof globalThis;

    try {
      const effectFns: (() => void)[] = [];
      internals.H = {
        useState: (init: unknown) => [init, () => {}],
        useRef: (init: unknown) => ({ current: init }),
        useCallback: (fn: unknown) => fn,
        useEffect: (fn: () => void) => { effectFns.push(fn); },
        useMemo: (fn: () => unknown) => fn(),
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const onClose = vi.fn();
      const portal = FxLangModal({
        target: { type: "move", name: "Thunderbolt" },
        onClose,
      });

      const portalAny = portal as unknown as {
        children: {
          props: {
            onPointerDown: (e: { stopPropagation: () => void }) => void;
            children: {
              props: {
                onPointerDown: (e: { stopPropagation: () => void }) => void;
              };
            };
          };
        };
      };

      // 1. Verify backdrop stops pointerdown
      const backdropStopPropagation = vi.fn();
      portalAny.children.props.onPointerDown({ stopPropagation: backdropStopPropagation });
      expect(backdropStopPropagation).toHaveBeenCalledTimes(1);

      // 2. Verify modal container stops pointerdown
      const modalStopPropagation = vi.fn();
      portalAny.children.props.children.props.onPointerDown({ stopPropagation: modalStopPropagation });
      expect(modalStopPropagation).toHaveBeenCalledTimes(1);

      // 3. Trigger effects and verify Escape key is captured with stopPropagation
      for (const fn of effectFns) fn();
      expect(keydownListener).toBeDefined();
      expect(captureMode).toBe(true);

      const escapeStopPropagation = vi.fn();
      keydownListener!({ key: "Escape", stopPropagation: escapeStopPropagation } as unknown as KeyboardEvent);
      expect(escapeStopPropagation).toHaveBeenCalledTimes(1);
      expect(onClose).toHaveBeenCalledTimes(1);
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
      globalThis.window = originalWindow;
    }
  });

  it("navigates to condition delegate on click and preserves previous tab on back", () => {
    let currentTargetState: any = { type: "move", name: "Fake Out" };
    let historyState: any[] = [];
    let activeTabState = "effects";

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

    const originalDocument = globalThis.document;
    const internals = (
      React as unknown as {
        __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE: { H: unknown };
      }
    ).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
    const prevH = internals.H;

    try {
      internals.H = {
        useState: (init: unknown) => {
          if (typeof init === "object" && init !== null && "type" in (init as any)) {
            return [currentTargetState, (updater: any) => {
              currentTargetState = typeof updater === "function" ? updater(currentTargetState) : updater;
            }];
          }
          if (Array.isArray(init)) {
            return [historyState, (updater: any) => {
              historyState = typeof updater === "function" ? updater(historyState) : updater;
            }];
          }
          if (init === "fxlang" || init === "effects") {
            return [activeTabState, (updater: any) => {
              activeTabState = typeof updater === "function" ? updater(activeTabState) : updater;
            }];
          }
          return [init, () => {}];
        },
        useRef: (init: unknown) => ({ current: init }),
        useCallback: (fn: unknown) => fn,
        useEffect: () => {},
        useMemo: (fn: () => unknown) => fn(),
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FxLangModal({
        target: { type: "move", name: "Fake Out" },
        onClose: () => {},
      });

      const portalAny = portal as unknown as {
        children: {
          props: {
            children: {
              props: {
                children: Array<{
                  props: Record<string, unknown>;
                }>;
              };
            };
          };
        };
      };

      // Find the code viewer container with onClick handler
      const content = portalAny.children.props.children.props.children[1];
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
      expect(currentTargetState).toEqual({ type: "condition", name: "flinch" });
      expect(historyState.length).toBe(1);
      expect(historyState[0].name).toBe("Fake Out");
      expect(historyState[0].tab).toBe("effects");
      expect(activeTabState).toBe("fxlang");
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
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

    const originalDocument = globalThis.document;
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
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FxLangModal({
        target: { type: "condition", name: "noretreat" },
        onClose: () => {},
      });

      const portalAny = portal as unknown as {
        children: {
          props: {
            children: {
              props: {
                children: Array<{
                  props: Record<string, unknown>;
                }>;
              };
            };
          };
        };
      };

      // Priority includes condition followed by move, ability, item, species
      expect(capturedOptions?.priority).toEqual([
        "condition",
        "move",
        "ability",
        "item",
        "species",
      ]);

      const header = portalAny.children.props.children.props.children[0];
      const headerStr = JSON.stringify(header);
      expect(headerStr).toContain("No Retreat");
      expect(headerStr).toContain("Move");
      expect(headerStr).toContain("fxlang");
      expect(headerStr).toContain("effects");
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
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

    const originalDocument = globalThis.document;
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
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FxLangModal({
        target: { type: "move", name: "Tackle" },
        onClose: () => {},
      });

      const portalAny = portal as unknown as {
        children: {
          props: {
            children: {
              props: {
                children: Array<{
                  props: Record<string, unknown>;
                }>;
              };
            };
          };
        };
      };

      const header = portalAny.children.props.children.props.children[0];
      const headerStr = JSON.stringify(header);
      expect(headerStr).toContain('"disabled":true');
      expect(headerStr).toContain("No fxlang callbacks defined");
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
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

    const originalDocument = globalThis.document;
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
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FxLangModal({
        target: { type: "move", name: "Horn Attack" },
        onClose: () => {},
      });

      const portalAny = portal as unknown as {
        children: {
          props: {
            children: {
              props: {
                children: Array<{
                  props: Record<string, unknown>;
                }>;
              };
            };
          };
        };
      };

      const header = portalAny.children.props.children.props.children[0];
      const headerStr = JSON.stringify(header);
      expect(headerStr).toContain("No fxlang callbacks defined");
      expect(headerStr).toContain("No structured effects defined");
      // Neither tab should be selected
      expect(headerStr).not.toContain('"aria-selected":true');
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
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
