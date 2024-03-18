import { getAppSetting, setAppSetting } from "@/Commands/Commands";
import { create } from "zustand";

export type ThemeType = "light" | "dark";

const getSystemPreferColorScheme = (delayUpdator: (theme: ThemeType) => void): ThemeType => {
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
}>((set) => ({
  theme: getSystemPreferColorScheme(theme => {
    document.body.setAttribute("arco-theme", theme);
    set({ theme });
  }),
  setTheme: (theme: ThemeType) => {
    document.body.setAttribute("arco-theme", theme);
    set({ theme });

    getAppSetting().then(setting => {
      setAppSetting({ ...setting, theme });
    });
  },
}));
