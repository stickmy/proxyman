import React, { FC, useLayoutEffect, useRef } from "react";
import { RequestConnection } from "@/Events/ConnectionEvents";
import { monaco } from "@/Monaco/Monaco";
import {
  isJson,
  tryStringifyWithSpaces,
} from "@/Components/Connections/Detail/Helper";
import dayjs from "dayjs";
import { Headers } from "@/Components/Connections/Detail/Headers";
import { Tabs } from "@arco-design/web-react";
import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";
import { useTheme } from "@/Components/TopBar/useTheme";

export const Request: FC<{
  request: RequestConnection;
}> = ({ request }) => {
  const { theme } = useTheme();

  const reqMonacoRef = useRef<monaco.editor.IStandaloneCodeEditor>();

  const isJsonReqBody = isJson(request.body);

  useLayoutEffect(() => {
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
    }, 200);
  }, []);

  return (
    <div className="req-res-panel">
      <div className="item">
        <span className="inline-block w-[60px] label">uri</span>
        <span className="value">{request.uri}</span>
      </div>
      <div className="item">
        <span className="inline-block w-[60px] label">method</span>
        <span className="value">{request.method}</span>
      </div>
      <div className="item">
        <span className="inline-block w-[60px] label">version</span>
        <span className="value">{request.version}</span>
      </div>
      <div className="item">
        <span className="inline-block w-[60px] label">time</span>
        <span className="value">
          {dayjs(request.time).format("YYYY-MM-DD HH:mm:ss:SSS")}
        </span>
      </div>
      <Tabs type="line" defaultActiveTab="header" justify style={{ height: "100%" }}>
        <Tabs.TabPane title="Header" key="header">
          <Headers headers={request.headers} />
        </Tabs.TabPane>
        <Tabs.TabPane title="Body" key="body">
          {request.body.length !== 0 ? (
            <div
              id="req-body"
              className="w-full h-full relative border border-gray4"
            ></div>
          ) : (
            <div>无内容</div>
          )}
        </Tabs.TabPane>
      </Tabs>
    </div>
  );
};
