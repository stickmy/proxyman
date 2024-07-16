import React, { FC, useLayoutEffect, useRef, useState } from "react";
import { RequestConnection } from "@/Events/ConnectionEvents";
import { monaco } from "@/Monaco/Monaco";
import {
  isJson,
  tryStringifyWithSpaces,
} from "@/Routes/Connections/Detail/Helper";
import dayjs from "dayjs";
import { Headers } from "@/Routes/Connections/Detail/Headers";
import { Tabs, Tab, Snippet } from "@nextui-org/react";
import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";
import { useTheme } from "@/Components/TopBar/useTheme";

export const Request: FC<{
  request: RequestConnection;
}> = ({ request }) => {
  const { theme } = useTheme();

  const [activeTab, setActiveTab] = useState<string | number>("header");

  const reqMonacoRef = useRef<monaco.editor.IStandaloneCodeEditor>();

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
  }, [activeTab]);

  return (
    <div className="flex flex-col h-full">
      <div className="px-1">
        <span className="inline-block w-[60px] text-tiny label">uri</span>
        <Snippet className="text-tiny" symbol="" size="sm">
          {request.uri}
        </Snippet>
      </div>
      <div className="px-1">
        <span className="inline-block w-[60px] text-tiny label">method</span>
        <span className="break-all text-tiny">{request.method}</span>
      </div>
      <div className="px-1">
        <span className="inline-block w-[60px] text-tiny label">version</span>
        <span className="break-all text-tiny">{request.version}</span>
      </div>
      <div className="px-1">
        <span className="inline-block w-[60px] text-tiny label">time</span>
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
          {request.body.length !== 0 ? (
            <div
              id="req-body"
              className="w-full h-full relative border border-gray4"
            ></div>
          ) : (
            <div>Empty</div>
          )}
        </Tab>
      </Tabs>
    </div>
  );
};
