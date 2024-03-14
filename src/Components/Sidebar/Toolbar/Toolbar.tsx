import React, { FC, useEffect, useState } from "react";
import { Input, Button, Message, Switch, Grid } from "@arco-design/web-react";
import {
  checkTlsCertInstalled,
  installTlsCert,
  getProxyStatus,
  startProxy,
  stopProxy,
} from "@/Commands/Commands";
import { useSystemProxy } from "./useSystemProxy";

const { Row, Col } = Grid;

const DEFAULT_PORT: string = "9000";

export const Toolbar: FC = () => {
  const [status, setStatus] = useState<boolean>(false);

  const [port, setPort] = useState<string>(DEFAULT_PORT);

  const {
    enableSystemProxy,
    turnOn: turnOnGlobalProxy,
    turnOff: turnOffGlobalProxy,
  } = useSystemProxy(port);

  const [installed, setInstalled] = useState<boolean>(false);
  const [installing, setInstalling] = useState<boolean>(false);

  async function start() {
    if (status) return;

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

  function onSwitchClick() {
    status ? stop() : start();
  }

  async function getStatus() {
    const status = await getProxyStatus();
    setStatus(status);
    return status;
  }

  async function checkCaInstalled() {
    const installed = await checkTlsCertInstalled();
    setInstalled(installed);
    return installed;
  }

  useEffect(() => {
    (async () => {
      const certInstalled = await checkCaInstalled();
      if (certInstalled) {
        const status = await getStatus();
        if (!status) {
          await start();
        }
      }
    })();
  }, []);

  async function installCa() {
    if (installing) return;

    setInstalling(true);

    Message.loading({
      id: "install_ca",
      content: "SLS 证书安装中",
    });

    try {
      const success = await installTlsCert();
      if (success) {
        Message.success({
          id: "install_ca",
          content: "安装成功",
        });
        setInstalled(true);
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

  return (
    <nav className="pb-2 mb-2 border-b-4 border-gray-50 border-solid bg-white">
      {!installed ? (
        <Button
          className="w-full"
          type="text"
          status="warning"
          onClick={installCa}
        >
          安装 SLS 证书
        </Button>
      ) : (
        <>
          <Row className="mb-2">
            <Col span={24}>
              <Input
                size="small"
                addBefore="监听端口号"
                value={port}
                onChange={setPort}
              />
            </Col>
          </Row>
          <Row>
            <Col span={11} offset={1}>
              <span className="mr-2">开启</span>
              <Switch size="small" checked={status} onChange={onSwitchClick} />
            </Col>
            <Col span={11} offset={1}>
              <span className="mr-2">全局代理</span>
              <Switch
                size="small"
                checked={enableSystemProxy}
                onChange={(value) => {
                  if (enableSystemProxy) {
                    void turnOffGlobalProxy();
                  } else {
                    void turnOnGlobalProxy();
                  }
                }}
              />
            </Col>
          </Row>
        </>
      )}
    </nav>
  );
};
