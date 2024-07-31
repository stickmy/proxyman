import type { ThemeType } from "@/Components/TopBar/useTheme";
import { invoke } from "@tauri-apps/api/core";

export const checkTlsCertInstalled = async () => {
  return invokeWithLogging<boolean>("check_cert_installed");
};

export const installTlsCert = async () => {
  return invokeWithLogging<boolean>("install_cert");
};

export const turnOnSystemProxy = async (port: string) => {
  return invokeWithLogging<boolean>("turn_on_global_proxy", {
    port,
  });
};

export const turnOffSystemProxy = async () => {
  return invokeWithLogging<boolean>("turn_off_global_proxy");
};

export const startProxy = async (port: number) => {
  return invokeWithLogging("start_proxy", { port });
};

export const stopProxy = async () => {
  return invokeWithLogging("stop_proxy");
};

export const getProxyStatus = async (): Promise<boolean> => {
  return invokeWithLogging<boolean>("proxy_status");
};

export const getValueList = async () => {
  return invokeWithLogging<string[]>("get_value_list");
};

export const getValueContent = async (name: string) => {
  return invokeWithLogging<string>("get_value_content", { name });
};

export const saveValue = async (name: string, value: string) => {
  return invokeWithLogging("save_value", { name, value });
};

export const removeValue = async (name: string) => {
  return invokeWithLogging("remove_value", { name });
};

export const getProcessorContent = async (
  packName: string,
  mode: string,
): Promise<string> => {
  return invokeWithLogging<string>("get_processor_content", {
    packName,
    mode,
  });
};

export const setProcessor = async (
  packName: string,
  mode: string,
  content: string,
) => {
  return invokeWithLogging("set_processor", {
    packName,
    mode,
    content,
  });
};

export interface ProcessorPackTransfer {
  packName: string;
  enable: boolean;
}

export const getProcessorPacks = async () => {
  return invokeWithLogging<ProcessorPackTransfer[]>("get_processor_packs");
};

export const addProcessPack = async (packName: string, enable?: boolean) => {
  return invokeWithLogging("add_processor_pack", {
    packName,
    enable: enable ?? false,
  });
};

export const removeProcessorPack = async (packName: string) => {
  return invokeWithLogging("remove_processor_pack", { packName });
};

export const updateProcessPackStatus = async (
  packName: string,
  enable: boolean,
) => {
  return invokeWithLogging("update_processor_pack_status", {
    packName,
    status: enable,
  });
};

export interface AppSetting {
  theme: ThemeType;
  layout: "bottom" | "right";
}

export const setAppSetting = async (setting: AppSetting) => {
  return invokeWithLogging("set_app_setting", {
    setting,
  });
};

export const getAppSetting = async () => {
  return invokeWithLogging<AppSetting>("get_app_setting");
};

const invokeWithLogging = async <T>(cmd: string, args?: any): Promise<T> => {
  console.debug("Invoke command - ", cmd, args);

  try {
    const t = await invoke<T>(cmd, args);
    console.debug("Command response - ", t);
    return t;
  } catch (error: any) {
    console.debug("Command error - ", error);
    throw typeof error === "string" ? error : JSON.parse(error);
  }
};
