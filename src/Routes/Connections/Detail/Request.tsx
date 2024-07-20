import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";
import { useTheme } from "@/Components/TopBar/useTheme";
import type { RequestConnection } from "@/Events/ConnectionEvents";
import type { monaco } from "@/Monaco/Monaco";
import { Headers } from "@/Routes/Connections/Detail/Headers";
import {
  isJson,
  tryStringifyWithSpaces,
} from "@/Routes/Connections/Detail/Helper";
import { Chip, Snippet, Tab, Tabs } from "@nextui-org/react";
import cls from "classnames";
import dayjs from "dayjs";
import React, { type FC, useLayoutEffect, useRef, useState } from "react";
import { useDecodeURIComponent } from "./Hooks/useDecodeURIComponent";

export const Request: FC<{
  request: RequestConnection;
}> = ({ request }) => {
  const { theme } = useTheme();

  const [activeTab, setActiveTab] = useState<string | number>("header");

  const reqMonacoRef = useRef<monaco.editor.IStandaloneCodeEditor>();

  const { isDecoded, decode } = useDecodeURIComponent(
    reqMonacoRef,
    request.body,
  );

  const isJsonReqBody = isJson(request.body);
  useLayoutEffect(() => {
    if (activeTab !== "body") return;

    setTimeout(() => {
      const target = document.getElementById("req-body");

      if (target) {
        reqMonacoRef.current = createMonacoEditor(target, {
          value: tryStringifyWithSpaces(request.body),
          language: isJsonReqBody ? "json" : undefined,
          readonly: true,
          theme,
        });
      }
    }, 100);
  }, [activeTab, isJsonReqBody, request.body]);

  return (
    <div className="flex flex-col h-full">
      <div className="px-1">
        <span className="inline-block w-16 text-tiny">uri</span>
        <Snippet className="text-tiny" hideSymbol size="sm">
          {request.uri}
        </Snippet>
      </div>
      <div className="px-1">
        <span className="inline-block w-16 text-tiny">method</span>
        <span className="break-all text-tiny">{request.method}</span>
      </div>
      <div className="px-1">
        <span className="inline-block w-16 text-tiny">version</span>
        <span className="break-all text-tiny">{request.version}</span>
      </div>
      <div className="px-1">
        <span className="inline-block w-16 text-tiny">time</span>
        <span className="break-all text-tiny">
          {dayjs(request.time).format("YYYY-MM-DD HH:mm:ss:SSS")}
        </span>
      </div>
      <Tabs
        size="sm"
        selectedKey={activeTab}
        onSelectionChange={setActiveTab}
        radius="full"
        className="mt-3"
        classNames={{
          panel: "h-full",
        }}
      >
        <Tab title="Header" key="header">
          <Headers headers={request.headers} />
        </Tab>
        <Tab title="Body" key="body">
          <div className="mb-[10px]">
            <Chip
              size="sm"
              className={cls(
                "cursor-pointer",
                "mr-1",
                isDecoded ? "bg-success-300" : "bg-default-100",
              )}
              onClick={decode}
            >
              decodeURIComponent
            </Chip>
          </div>
          {request.body.length !== 0 ? (
            <div id="req-body" className="w-full h-full relative" />
          ) : (
            <div>Empty</div>
          )}
        </Tab>
      </Tabs>
    </div>
  );
};
