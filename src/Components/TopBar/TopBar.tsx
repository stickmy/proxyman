import { Button, Input, Message, Tooltip } from "@arco-design/web-react";
import { MouseEvent, useEffect, useState } from "react";
import {
  checkTlsCertInstalled,
  getProxyStatus,
  installTlsCert,
  startProxy,
  stopProxy,
} from "@/Commands/Commands";
import {
  IconBrush,
  IconCopy,
  IconDesktop,
  IconMoon,
  IconPause,
  IconPlayArrow,
  IconSafe,
  IconSun,
} from "@arco-design/web-react/icon";
import { useSystemProxy } from "@/Components/Sidebar/Toolbar/useSystemProxy";
import "./topbar.css";
import { useConnectionStore } from "@/Store/ConnectionStore";
import { useTheme } from "@/Components/TopBar/useTheme";

const DEFAULT_PORT: string = "9000";

export const TopBar = () => {
  const { theme, setTheme } = useTheme();
  const { clearConnections } = useConnectionStore();
  const [status, setStatus] = useState<boolean>(false);

  const [port, setPort] = useState<string>(DEFAULT_PORT);

  const [caInstalled, setCaInstalled] = useState<boolean>(false);
  const [installing, setInstalling] = useState<boolean>(false);

  const {
    enableSystemProxy,
    turnOn: turnOnSystemProxy,
    turnOff: turnOffSystemProxy,
  } = useSystemProxy(port);

  function onPortChange(value: string) {
    if (status) {
      Message.info(`请先停止代理服务, 再修改端口号`);
      return;
    }

    setPort(value);
  }

  async function getStatus() {
    const status = await getProxyStatus();
    setStatus(status);
    return status;
  }

  async function checkTlsInstalled() {
    const installed = await checkTlsCertInstalled();
    setCaInstalled(installed);
    return installed;
  }

  async function installCa() {
    if (caInstalled) return;
    if (installing) return;

    setInstalling(true);

    Message.loading({
      id: "install_ca",
      content: "TLS 证书安装中",
    });

    try {
      const success = await installTlsCert();
      if (success) {
        Message.success({
          id: "install_ca",
          content: "安装成功",
        });
        setCaInstalled(true);
      } else {
        Message.error({
          id: "install_ca",
          content: "安装失败",
        });
      }
    } catch (error: any) {
      Message.error({
        id: "install_ca",
        content: error,
      });
    } finally {
      setInstalling(false);
    }
  }

  async function start() {
    if (status) return;

    const caInstalled = await checkTlsInstalled();
    if (!caInstalled) {
      Message.info(`请先安装 TLS 证书`);
      return;
    }

    try {
      await startProxy(parseInt(port));
      setStatus(true);
    } catch (error: any) {
      Message.error({
        content: error,
        duration: 3000,
      });
    }
  }

  async function stop() {
    await stopProxy();
    setStatus(false);
  }

  useEffect(() => {
    void checkTlsInstalled();
  }, []);

  function copyTerminalProxyCmd(evt: MouseEvent) {
    evt.stopPropagation();

    navigator.clipboard
      .writeText(`export https_proxy=http://127.0.0.1:${port};`)
      .then(() => {
        Message.success(`已成功复制到剪贴板`);
      })
      .catch((err) => {
        Message.error(`复制到剪贴板失败`);
      });
  }

  return (
    <div className="pl-2 pr-2 pb-2 pt-2 flex flex-row items-center top-bar">
      <Input
        size="small"
        className="address"
        addBefore="代理地址"
        value={port}
        onChange={onPortChange}
        prefix={<span>127.0.0.1:</span>}
        suffix={
          <Tooltip mini content="复制终端代理命令">
            <IconCopy
              className="copy-terminal"
              onClick={copyTerminalProxyCmd}
            />
          </Tooltip>
        }
      />
      <div className="mr-4 ml-4 shrink-0">
        <Tooltip
          mini
          content={theme === "light" ? "切换暗黑模式" : "切换亮色模式"}
        >
          <div className="sys-setting-wrap">
            {theme === "light" ? (
              <IconSun
                className="sys-setting"
                data-enable={true}
                onClick={() => setTheme("dark")}
              />
            ) : (
              <IconMoon
                className="sys-setting"
                data-enable={true}
                onClick={() => setTheme("light")}
              />
            )}
          </div>
        </Tooltip>
        <Tooltip
          mini
          content={enableSystemProxy ? "系统代理已开启" : "系统代理未开启"}
        >
          <div className="sys-setting-wrap">
            <IconDesktop
              className="sys-setting"
              data-enable={enableSystemProxy}
              onClick={() => {
                if (enableSystemProxy) {
                  void turnOffSystemProxy();
                } else {
                  void turnOnSystemProxy();
                }
              }}
            />
          </div>
        </Tooltip>
        <Tooltip
          mini
          content={caInstalled ? "TLS 证书已安装" : "TLS 证书未安装"}
        >
          <div className="sys-setting-wrap">
            <IconSafe
              className="sys-setting"
              data-enable={caInstalled}
              onClick={installCa}
            />
          </div>
        </Tooltip>
        <Tooltip mini content="清空请求数据">
          <div className="sys-setting-wrap">
            <IconBrush className="sys-setting" onClick={clearConnections} />
          </div>
        </Tooltip>
      </div>
      {status ? (
        <Button
          icon={<IconPause />}
          className="proxy-btn proxy-stop"
          onClick={stop}
        >
          停止
        </Button>
      ) : (
        <Button
          icon={<IconPlayArrow />}
          className="proxy-btn proxy-start"
          onClick={start}
        >
          开启
        </Button>
      )}
    </div>
  );
};
