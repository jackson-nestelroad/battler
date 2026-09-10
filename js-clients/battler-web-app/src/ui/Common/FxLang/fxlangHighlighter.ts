import fxlangGrammar from "../../../../../../fxlang-ext/syntaxes/fxlang-injection.tmLanguage.json";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let highlighterPromise: Promise<any> | null = null;

async function getHighlighterInstance() {
  if (!highlighterPromise) {
    highlighterPromise = (async () => {
      const { createHighlighter } = await import("shiki");
      return createHighlighter({
        themes: ["dark-plus"],
        langs: [
          "json",
          {
            ...(fxlangGrammar as Record<string, unknown>),
            injectTo: ["source.json"],
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          } as any,
        ],
      });
    })();
  }
  return highlighterPromise;
}

/**
 * Highlights formatted JSON code containing fxlang AST expressions
 * with VS Code Dark+ theme parity using fxlang-injection grammar.
 */
export async function highlightFxlangJson(jsonCode: string): Promise<string> {
  const highlighter = await getHighlighterInstance();
  return highlighter.codeToHtml(jsonCode, {
    lang: "json",
    theme: "dark-plus",
  });
}
