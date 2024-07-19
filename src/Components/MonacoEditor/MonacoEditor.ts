import type { ThemeType } from "@/Components/TopBar/useTheme";
import { monaco } from "@/Monaco/Monaco";

export const createMonacoEditor = (
  elem: HTMLElement,
  options: {
    value?: string;
    language?: string;
    theme?: ThemeType;
    readonly?: boolean;
    lineNumbers?: "on" | "off";
  },
) => {
  const {
    value,
    language,
    readonly,
    theme = "light",
    lineNumbers = "off",
  } = options;

  if (!elem.hasAttribute("data-selectable")) {
    elem.setAttribute("data-selectable", "true");
  }

  return monaco.editor.create(elem, {
    value,
    language,
    lineNumbers,
    inlayHints: {
      enabled: "off",
    },
    minimap: {
      enabled: false,
    },
    theme: transformSystemThemeToMonaco(theme),
    scrollbar: {
      verticalScrollbarSize: 4,
    },
    readOnly: readonly,
  });
};

const transformSystemThemeToMonaco = (theme: ThemeType) => {
  return theme === "light" ? "vs" : "vs-dark";
};
