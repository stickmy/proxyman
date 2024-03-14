import { invoke, InvokeArgs } from "@tauri-apps/api/tauri";

export const checkTlsCertInstalled = async () => {
  return invokeWithLogging<boolean>("plugin:proxy|check_cert_installed");
};

export const installTlsCert = async () => {
  return invokeWithLogging<boolean>("plugin:proxy|install_cert");
};

export const turnOnSystemProxy = async (port: string) => {
  return invokeWithLogging<boolean>("plugin:proxy|turn_on_global_proxy", {
    port,
  });
};

export const turnOffSystemProxy = async () => {
  return invokeWithLogging<boolean>("plugin:proxy|turn_off_global_proxy");
};

export const startProxy = async (port: number) => {
  return invokeWithLogging("plugin:proxy|start_proxy", { port });
};

export const stopProxy = async () => {
  return invokeWithLogging("plugin:proxy|stop_proxy");
};

export const getProxyStatus = async (): Promise<boolean> => {
  return invokeWithLogging<boolean>("plugin:proxy|proxy_status");
};

export const getProcessorContent = async (mode: string): Promise<string> => {
  return invokeWithLogging<string>("plugin:proxy|get_processor_content", {
    mode,
  });
};

export const setProcessor = async (mode: string, content: string) => {
  return invokeWithLogging("plugin:proxy|set_processor", {
    mode,
    content,
  });
};

export const removeResponseMapping = async (req: string) => {
  return invokeWithLogging("plugin:proxy|remove_response_mapping", {
    req,
  });
};

const invokeWithLogging = async <T>(
  cmd: string,
  args?: InvokeArgs
): Promise<T> => {
  console.debug("Invoke command - ", cmd, args);

  try {
    const t = await invoke<T>(cmd, args);
    console.debug("Command response - ", t);
    return t;
  } catch (error: any) {
    console.debug("Command error - ", error);
    throw JSON.parse(error);
  }
};
