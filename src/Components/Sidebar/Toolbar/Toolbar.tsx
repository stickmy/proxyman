import React, { FC, useEffect, useState } from "react";
import { Input, Button, Message, Tooltip } from "@arco-design/web-react";
import { IconPause, IconPlayArrow } from "@arco-design/web-react/icon";
import { GlobeIcon } from "@radix-ui/react-icons";
import {
  checkCertInstalled,
  installCert,
  getProxyStatus,
  startProxy,
  stopProxy,
} from "@/Commands/Commands";
import { useGlobalProxy } from "./useGlobalProxy";

export const Toolbar: FC = () => {
  const [status, setStatus] = useState<boolean>(false);

  const [port, setPort] = useState<string>("9000");

  const {
    globally,
    turnOn: turnonGlobalProxy,
    turnOff: turnoffGlobalProxy,
  } = useGlobalProxy(port);

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
    const installed = await checkCertInstalled();
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
      content: "Installing",
    });

    try {
      const success = await installCert();
      if (success) {
        Message.success({
          id: "install_ca",
          content: "Install success",
        });
        setInstalled(true);
      } else {
        Message.error({
          id: "install_ca",
          content: "Install failed",
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
    <nav className="flex flex-row items-center">
      {!installed ? (
        <Button
          className="w-full"
          type="text"
          status="warning"
          onClick={installCa}
        >
          Install certificate for https decoding
        </Button>
      ) : (
        <>
          <Input
            size="small"
            prefix="listening on port:"
            value={port}
            onChange={setPort}
          />
          <Button
            className="ml-[8px] shrink-0"
            icon={
              status ? (
                <IconPause className="text-yellow-500" />
              ) : (
                <IconPlayArrow className="text-green10" />
              )
            }
            onClick={onSwitchClick}
          />
          <Button
            className="ml-[8px] shrink-0"
            onClick={() => {
              if (globally) {
                turnoffGlobalProxy();
              } else {
                turnonGlobalProxy();
              }
            }}
            icon={
              <Tooltip
                content={
                  globally ? "Turn off global proxy" : "Set as global proxy"
                }
                mini
              >
                <GlobeIcon
                  className={`inline-block ${globally ? "text-green10" : ""}`}
                />
              </Tooltip>
            }
          ></Button>
        </>
      )}
    </nav>
  );
};
