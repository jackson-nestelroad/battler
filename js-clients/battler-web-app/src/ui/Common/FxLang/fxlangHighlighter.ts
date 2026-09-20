import type { HighlighterCore, LanguageRegistration } from "shiki";
import fxlangGrammar from "../../../../../../fxlang-ext/syntaxes/fxlang-injection.tmLanguage.json";

let highlighterPromise: Promise<HighlighterCore> | null = null;

async function getHighlighterInstance(): Promise<HighlighterCore> {
  if (!highlighterPromise) {
    highlighterPromise = (async () => {
      const [{ createHighlighterCore }, { createOnigurumaEngine }, darkPlus, json] =
        await Promise.all([
          import("shiki/core"),
          import("shiki/engine/oniguruma"),
          import("shiki/themes/dark-plus.mjs"),
          import("shiki/langs/json.mjs"),
        ]);
      return createHighlighterCore({
        themes: [darkPlus.default],
        langs: [
          json.default,
          {
            ...(fxlangGrammar as unknown as LanguageRegistration),
            injectTo: ["source.json"],
          },
        ],
        engine: createOnigurumaEngine(import("shiki/wasm")),
      });
    })().catch((err) => {
      highlighterPromise = null;
      throw err;
    });
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

