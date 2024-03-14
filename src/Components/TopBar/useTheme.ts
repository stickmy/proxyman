import { create } from "zustand";

export type ThemeType = "light" | "dark";

const getSystemPreferColorScheme = (): ThemeType => {
  return window.matchMedia("screen and (prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
};

export const useTheme = create<{
  theme: ThemeType;
  setTheme: (theme: ThemeType) => void;
}>((set) => ({
  theme: getSystemPreferColorScheme(),
  setTheme: (theme: ThemeType) => {
    document.body.setAttribute("arco-theme", theme);
    set({ theme });
  },
}));
