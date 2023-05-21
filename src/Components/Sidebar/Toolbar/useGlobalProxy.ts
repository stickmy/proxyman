import { turnOffGlobalProxy, turnOnGlobalProxy } from "@/Commands/Commands";
import { Notification } from "@arco-design/web-react";
import { useState } from "react";

export const useGlobalProxy = (port: string) => {
  const [globally, setGlobally] = useState<boolean>(false);
  const [switching, setSwitching] = useState<boolean>(false);

  async function turnOn() {
    if (switching) return;

    try {
      const ret = await turnOnGlobalProxy(port);
      if (ret) {
        setGlobally(true);
      }

      if (!ret) {
        Notification.warning({ content: "Set as global proxy failed" });
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
      const ret = await turnOffGlobalProxy();
      if (ret) {
        setGlobally(false);
      }

      if (!ret) {
        Notification.warning({ content: "Turnoff global proxy failed" });
      }
    } catch (error: any) {
      Notification.warning({ content: error });
    } finally {
      setSwitching(false);
    }
  }

  return {
    globally,
    turnOn,
    turnOff,
  };
};
