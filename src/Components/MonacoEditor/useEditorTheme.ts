import { type ThemeType, useTheme } from "@/Components/TopBar/useTheme";
import type { monaco } from "@/Monaco/Monaco";
import { type MutableRefObject, useEffect } from "react";

export const useEditorTheme = (
  ref: MutableRefObject<monaco.editor.IStandaloneCodeEditor | undefined>,
) => {
  const { theme, onThemeChange, offThemeChange } = useTheme();

  const setEditorTheme = (theme: ThemeType) => {
    if (ref.current) {
      ref.current.updateOptions({
        theme: theme === "light" ? "vs" : "vs-dark",
      });
    }
  };

  useEffect(() => {
    onThemeChange(setEditorTheme);

    return () => {
      offThemeChange(setEditorTheme);
    };
  }, []);

  return {
    theme,
  };
};
