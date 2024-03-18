import { turnOffSystemProxy, turnOnSystemProxy } from "@/Commands/Commands";
import { Notification } from "@arco-design/web-react";
import { useState } from "react";

export const useSystemProxy = (port: string) => {
  const [enableSystemProxy, setEnableSystemProxy] = useState<boolean>(false);
  const [switching, setSwitching] = useState<boolean>(false);

  async function turnOn() {
    if (switching) return;

    try {
      const ret = await turnOnSystemProxy(port);
      if (ret) {
        setEnableSystemProxy(true);
      }

      if (!ret) {
        Notification.warning({ content: "系统代理开启失败" });
      }
    } catch (error: any) {
      Notification.warning({ content: error });
    } finally {
      setSwitching(false);
    }
  }

  async function turnOff() {
    if (switching) return;

    try {
      const ret = await turnOffSystemProxy();
      if (ret) {
        setEnableSystemProxy(false);
      }

      if (!ret) {
        Notification.warning({ content: "系统代理关闭失败" });
      }
    } catch (error: any) {
      Notification.warning({ content: error });
    } finally {
      setSwitching(false);
    }
  }

  return {
    enableSystemProxy,
    turnOn,
    turnOff,
  };
};
