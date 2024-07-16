import { AppSetting, getAppSetting, setAppSetting } from "@/Commands/Commands";
import { create } from "zustand";

const readAppSetting = (
  delayUpdator: (setting: AppSetting) => void
): AppSetting => {
  getAppSetting().then((setting) => delayUpdator(setting));

  return {
    theme: "dark",
    layout: "right",
  };
};

export const useAppSetting = create<{
  appSetting: AppSetting;
  setAppSetting: (setting: Partial<AppSetting>) => void;
}>((set, get) => ({
  appSetting: readAppSetting((appSetting) => set({ appSetting })),
  setAppSetting: (appSetting: Partial<AppSetting>) => {
    set({ appSetting: { ...get().appSetting, ...appSetting } });

    getAppSetting().then((setting) => {
      setAppSetting({ ...setting, ...appSetting });
    });
  },
}));
