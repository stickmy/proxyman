import { getAppSetting, setAppSetting } from "@/Commands/Commands";
import { create } from "zustand";

export type ThemeType = "light" | "dark";

const getSystemPreferColorScheme = (
  delayUpdator: (theme: ThemeType) => void,
): ThemeType => {
  // 初始化返回系统偏好, 随后采用用户设置中的主题配置
  getAppSetting().then(setting => {
    delayUpdator(setting.theme);
  });

  return window.matchMedia("screen and (prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
};

export const useTheme = create<{
  theme: ThemeType;
  setTheme: (theme: ThemeType) => void;
  onThemeChange: (fn: (theme: ThemeType) => void) => void;
  offThemeChange: (fn: (theme: ThemeType) => void) => void;
}>(set => ({
  theme: getSystemPreferColorScheme(theme => {
    document.body.className = `${theme} text-foreground bg-background`;
    set({ theme });
  }),
  setTheme: (theme: ThemeType) => {
    document.body.className = `${theme} text-foreground bg-background`;
    set({ theme });

    for (const fn of themeListeners) {
      fn(theme);
    }

    getAppSetting().then(setting => {
      setAppSetting({ ...setting, theme });
    });
  },
  onThemeChange: fn => {
    for (const listener of themeListeners) {
      if (listener === fn) return;
    }

    themeListeners.push(fn);
  },
  offThemeChange: fn => {
    for (let i = 0; i < themeListeners.length; i++) {
      if (themeListeners[i] === fn) {
        themeListeners.splice(i, 1);
      }
    }
  },
}));

const themeListeners: ((theme: ThemeType) => void)[] = [];
