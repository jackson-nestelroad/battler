import { describe, expect, it } from "vitest";
import { isTargetInsideModal, toElement } from "./dom";

describe("dom utils", () => {
  describe("toElement", () => {
    it("returns null for null, undefined, or non-element targets", () => {
      expect(toElement(null)).toBeNull();
      expect(toElement(undefined as unknown as EventTarget)).toBeNull();
      expect(toElement({} as unknown as EventTarget)).toBeNull();
    });

    it("resolves duck-typed elements with closest method", () => {
      const mockEl = { closest: () => null } as unknown as EventTarget;
      expect(toElement(mockEl)).toBe(mockEl);
    });

    it("resolves elements with nodeType === 1 directly", () => {
      const mockElement = { nodeType: 1, tagName: "DIV", closest: () => null } as unknown as EventTarget;
      expect(toElement(mockElement)).toBe(mockElement);
    });

    it("resolves text nodes with nodeType === 3 to their parent element", () => {
      const parent = { nodeType: 1, tagName: "DIV", closest: () => null };
      const textNode = { nodeType: 3, parentElement: parent } as unknown as EventTarget;
      expect(toElement(textNode)).toBe(parent);
    });

    it("resolves text nodes to their parent element via Node mock", () => {
      class MockNode {}
      const originalNode = globalThis.Node;
      // @ts-expect-error test mock
      globalThis.Node = MockNode;

      try {
        const parent = { closest: () => null };
        const textNode = new MockNode() as unknown as { parentElement: typeof parent };
        textNode.parentElement = parent;
        expect(toElement(textNode as unknown as EventTarget)).toBe(parent);
      } finally {
        globalThis.Node = originalNode;
      }
    });
  });

  describe("isTargetInsideModal", () => {
    it("returns false for null, undefined, or non-element targets", () => {
      expect(isTargetInsideModal(null)).toBe(false);
      expect(isTargetInsideModal(undefined as unknown as EventTarget)).toBe(false);
      expect(isTargetInsideModal({} as unknown as EventTarget)).toBe(false);
    });

    it("returns true when target or ancestor has role='dialog' or aria-modal='true'", () => {
      const dialogChild = {
        closest: (selector: string) => (selector.includes('[role="dialog"]') ? {} : null),
      };
      expect(isTargetInsideModal(dialogChild as unknown as EventTarget)).toBe(true);

      const modalChild = {
        closest: (selector: string) => (selector.includes('[aria-modal="true"]') ? {} : null),
      };
      expect(isTargetInsideModal(modalChild as unknown as EventTarget)).toBe(true);

      const regularElement = {
        closest: () => null,
      };
      expect(isTargetInsideModal(regularElement as unknown as EventTarget)).toBe(false);
    });

    it("handles text nodes nested inside dialog elements", () => {
      class MockNode {}
      const originalNode = globalThis.Node;
      // @ts-expect-error test mock
      globalThis.Node = MockNode;

      try {
        const textNodeInModal = new MockNode() as unknown as {
          parentElement: { closest: (sel: string) => Record<string, unknown> | null };
        };
        textNodeInModal.parentElement = {
          closest: (selector: string) => (selector.includes('[role="dialog"]') ? {} : null),
        };
        expect(isTargetInsideModal(textNodeInModal as unknown as EventTarget)).toBe(true);

        const textNodeOutside = new MockNode() as unknown as {
          parentElement: { closest: (sel: string) => null };
        };
        textNodeOutside.parentElement = { closest: () => null };
        expect(isTargetInsideModal(textNodeOutside as unknown as EventTarget)).toBe(false);
      } finally {
        globalThis.Node = originalNode;
      }
    });
  });
});

