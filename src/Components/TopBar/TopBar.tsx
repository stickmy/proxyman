import toast from "react-hot-toast";
import { Input, Button, Tooltip } from "@nextui-org/react";
import { MouseEvent, useEffect, useState, ChangeEvent } from "react";
import {
  checkTlsCertInstalled,
  getProxyStatus,
  installTlsCert,
  startProxy,
  stopProxy,
} from "@/Commands/Commands";
import { useSystemProxy } from "@/Components/TopBar/useSystemProxy";
import { useConnectionStore } from "@/Store/ConnectionStore";
import { useTheme } from "@/Components/TopBar/useTheme";
import {
  CopyIcon,
  SunIcon,
  MoonIcon,
  NetworkIcon,
  SecureIcon,
  ClearIcon,
  PauseIcon,
  PlayIcon,
  RightLayoutIcon,
  BottomLayoutIcon,
} from "@/Icons";
import { useLayout } from "./useLayout";
import "./index.css";

const DEFAULT_PORT: string = "9000";

export const TopBar = () => {
  const { theme, setTheme } = useTheme();
  const [layout, setLayout] = useLayout();

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

  function onPortChange(event: ChangeEvent<HTMLInputElement>) {
    if (status) {
      toast(`请先停止代理服务, 再修改端口号`);
      return;
    }

    setPort(event.target.value);
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

    const installTlsDefer = installTlsCert();

    toast.promise(installTlsDefer, {
      loading: "TLS 证书安装中",
      success: "安装成功",
      error: "安装失败",
    });

    try {
      const success = await installTlsDefer;
      if (success) {
        setCaInstalled(true);
      }
    } finally {
      setInstalling(false);
    }
  }

  async function start() {
    if (status) return;

    const caInstalled = await checkTlsInstalled();
    if (!caInstalled) {
      toast(`请先安装 TLS 证书`);
      return;
    }

    try {
      await startProxy(parseInt(port));
      setStatus(true);
    } catch (error: any) {
      toast.error(error, {
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
        toast.success(`终端代理命令已复制`);
      })
      .catch((err) => {
        toast.error(`复制失败`);
      });
  }

  return (
    <div className="pl-2 pr-2 pb-2 pt-2 flex flex-row items-center bg-content1">
      <span className="flex-shrink-0 text-tiny text-default-400 mr-2">
        代理地址
      </span>
      <Input
        classNames={{
          input: ["text-tiny"],
        }}
        radius="sm"
        size="sm"
        value={port}
        onChange={onPortChange}
        startContent={
          <div className="pointer-events-none flex items-center">
            <span className="text-default-400 text-tiny">127.0.0.1:</span>
          </div>
        }
        endContent={
          <div
            className="flex items-center pl-2 cursor-pointer"
            onClick={copyTerminalProxyCmd}
          >
            <CopyIcon size={12} />
          </div>
        }
      />
      <div className="flex mr-4 ml-4 shrink-0">
        <Tooltip
          size="sm"
          content={theme === "light" ? "切换暗黑模式" : "切换亮色模式"}
        >
          <div className="sys-setting-wrap">
            {theme === "light" ? (
              <MoonIcon
                className="sys-setting cursor-pointer"
                onClick={() => setTheme("dark")}
              />
            ) : (
              <SunIcon
                className="sys-setting cursor-pointer"
                onClick={() => setTheme("light")}
              />
            )}
          </div>
        </Tooltip>
        <Tooltip
          size="sm"
          content={enableSystemProxy ? "系统代理已开启" : "系统代理未开启"}
        >
          <div className="sys-setting-wrap">
            <NetworkIcon
              className="sys-setting cursor-pointer"
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
          size="sm"
          content={caInstalled ? "TLS 证书已安装" : "TLS 证书未安装"}
        >
          <div className="sys-setting-wrap">
            <SecureIcon
              className="sys-setting cursor-pointer"
              data-enable={caInstalled}
              onClick={installCa}
            />
          </div>
        </Tooltip>
        <Tooltip
          size="sm"
          content={
            layout === "bottom" ? "切换至详情右侧布局" : "切换至详情底部布局"
          }
        >
          <div className="sys-setting-wrap">
            {layout === "bottom" ? (
              <RightLayoutIcon
                className="sys-setting cursor-pointer"
                onClick={() => setLayout("right")}
              />
            ) : (
              <BottomLayoutIcon
                className="sys-setting cursor-pointer"
                onClick={() => setLayout("bottom")}
              />
            )}
          </div>
        </Tooltip>
        <Tooltip size="sm" content="清空请求数据">
          <div className="sys-setting-wrap">
            <ClearIcon
              className="sys-setting cursor-pointer"
              onClick={clearConnections}
            />
          </div>
        </Tooltip>
      </div>
      {status ? (
        <Button
          size="md"
          className="h-8"
          variant="solid"
          startContent={
            <PauseIcon size={24} className="text-primary-foreground" />
          }
          onClick={stop}
        >
          停止
        </Button>
      ) : (
        <Button
          size="sm"
          className="h-8"
          variant="solid"
          color="primary"
          startContent={
            <PlayIcon size={32} className="text-primary-foreground" />
          }
          onClick={start}
        >
          开启
        </Button>
      )}
    </div>
  );
};
