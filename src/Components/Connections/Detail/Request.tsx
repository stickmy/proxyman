import React, { FC, useLayoutEffect, useRef } from "react";
import { RequestConnection } from "@/Events/ConnectionEvents";
import { monaco } from "@/Monaco/Monaco";
import {
  isJson,
  tryStringifyWithSpaces,
} from "@/Components/Connections/Detail/Helper";
import { LabelSeparator } from "@/Components/Connections/Detail/Separator";
import dayjs from "dayjs";

export const Request: FC<{
  request: RequestConnection;
}> = ({ request }) => {
  const reqMonacoRef = useRef<monaco.editor.IStandaloneCodeEditor>();

  const isJsonReqBody = isJson(request.body);

  useLayoutEffect(() => {
    const target = document.getElementById("req-body");

    if (target) {
      if (!reqMonacoRef.current) {
        reqMonacoRef.current = monaco.editor.create(target, {
          value: tryStringifyWithSpaces(request.body),
          language: isJsonReqBody ? "json" : undefined,
          lineNumbers: "off",
          minimap: {
            enabled: false,
          },
        });
      }
    }
  }, []);
  return (
    <div>
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
      <LabelSeparator label="Headers" />
      {Object.keys(request.headers).map((key) => (
        <div className="item" key={key}>
          <span className="inline-block mr-2 label">{key}</span>
          <span className="value">{request.headers[key]}</span>
        </div>
      ))}
      <LabelSeparator label="Body" />
      {request.body.length !== 0 && (
        <div id="req-body" className="w-full h-[500px] relative"></div>
      )}
    </div>
  );
};
