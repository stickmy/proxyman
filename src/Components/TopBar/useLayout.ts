import { AppSetting } from "@/Commands/Commands";
import { useAppSetting } from "@/Store/useAppSetting";

export type LayoutType = AppSetting["layout"];

export const useLayout = (): [LayoutType, (layout: LayoutType) => void] => {
  const { appSetting, setAppSetting } = useAppSetting();

  const setLayout = (layout: LayoutType) => {
    setAppSetting({ layout });
  };

  return [appSetting.layout, setLayout];
};
