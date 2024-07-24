import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";
import { useTheme } from "@/Components/TopBar/useTheme";
import { type ResponseConnection, RuleMode } from "@/Events/ConnectionEvents";
import type { monaco } from "@/Monaco/Monaco";
import { Headers } from "@/Routes/Connections/Detail/Headers";
import { isJsonp } from "@/Routes/Connections/Detail/Helper";
import {
  Chip,
  Snippet,
  Tab,
  Tabs,
  Tooltip,
  useDisclosure,
} from "@nextui-org/react";
import cls from "classnames";
import dayjs from "dayjs";
import React, { type FC, useLayoutEffect, useRef, useState } from "react";
import { usePretty } from "./Hooks/usePretty";
import { CreateValue } from "@/Routes/Value/CreateValue";
import { useNavigate } from "react-router-dom";

export const Response: FC<{
  response: ResponseConnection;
}> = ({ response }) => {
  const navigate = useNavigate();

  const { theme } = useTheme();

  const [activeTab, setActiveTab] = useState<string | number>("body");

  const resMonacoRef = useRef<monaco.editor.IStandaloneCodeEditor>();

  const { isPretty, pretty } = usePretty(resMonacoRef, response.body);

  const contentTypeKey = Object.keys(response.headers).find(
    (x) => x.toLowerCase() === "content-type",
  );
  const contentType = contentTypeKey
    ? response.headers[contentTypeKey]
    : undefined;

  useLayoutEffect(() => {
    if (activeTab !== "body") return;

    setTimeout(() => {
      const target = document.getElementById("res-body");

      if (target) {
        const { body, language } = getBodyModel(response);

        resMonacoRef.current = createMonacoEditor(target, {
          value: body,
          readonly: true,
          lineNumbers: "on",
          language,
          theme,
        });
      }
    }, 100);
  }, [activeTab]);

  const [savedResponseValue, setSavedResponseValue] = useState<string>();
  const { isOpen, onOpenChange, onOpen } = useDisclosure();
  const [beEditing, setBeEditing] = useState<boolean>(false);

  const onEditClick = () => {
    // 没处于编辑状态, 开启编辑状态
    if (!beEditing) {
      // 编辑行为分两种
      // 1. 第一次进入编辑态
      // 2. 之前已经保存为值文件, 并通过 Response 处理器返回为这次的响应, 此时的编辑是跳转到值文件
      if (response.effects) {
        const effects = Object.values(response.effects).flat();
        const effect = effects.find((x) => x.name === RuleMode.Response);

        if (effect) {
          navigate(`/value/${effect.info.name}`);
          return;
        }
      }

      if (!resMonacoRef.current) return;

      resMonacoRef.current.updateOptions({
        readOnly: beEditing,
      });
    } else {
      // 出于编辑状态, 保存值
      if (!resMonacoRef.current) return;

      const body = resMonacoRef.current.getValue();
      if (body === response.body) return;

      setSavedResponseValue(asResponseValue(response, body));

      onOpen();
    }

    setBeEditing(!beEditing);
  };

  const notStringLike = !!(
    contentType &&
    ["image", "octet-stream", "media"].some((type) =>
      contentType.includes(type),
    )
  );

  return (
    <div className="flex flex-col h-full">
      <div className="px-1">
        <Tooltip
          placement="top-start"
          content="跟 request uri 的区别是: 它是一系列的规则处理之后的值, 是实际发生请求的 uri"
        >
          <span className="inline-block w-16 text-tiny">uri</span>
        </Tooltip>
        <Snippet className="text-tiny" hideSymbol size="sm">
          {response.uri}
        </Snippet>
      </div>
      <div className="px-1">
        <span className="inline-block w-16 text-tiny">version</span>
        <span className="break-all text-tiny">{response.version}</span>
      </div>
      <div className="px-1">
        <span className="inline-block w-16 text-tiny">status</span>
        <span className="break-all text-tiny">{response.status}</span>
      </div>
      <div className="px-1">
        <span className="inline-block w-16 text-tiny">time</span>
        <span className="break-all text-tiny">
          {dayjs(response.time).format("YYYY-MM-DD HH:mm:ss:SSS")}
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
          <Headers headers={response.headers} />
        </Tab>
        <Tab title="Body" key="body">
          <div className="mb-[10px]">
            <Chip
              onClick={pretty}
              size="sm"
              className={cls(
                "mr-2 cursor-pointer px-2",
                "hover:bg-default-200",
                isPretty ? "bg-success-300" : "bg-default-100",
              )}
            >
              格式化
            </Chip>
            <Chip
              onClick={onEditClick}
              size="sm"
              className={cls(
                "mr-2 cursor-pointer px-2",
                "hover:bg-default-200",
                beEditing ? "bg-success-300" : "bg-default-100",
              )}
            >
              {beEditing ? "保存为值文件" : "编辑"}
            </Chip>
          </div>
          {notStringLike && (
            <MediaResponse uri={response.uri} contentType={contentType} />
          )}
          <div
            id="res-body"
            className="w-full h-full relative border border-transparent data-[editing=true]:border-dashed data-[editing=true]:border-blue9"
            data-editing={beEditing}
            style={{
              display:
                !notStringLike && response.body.length !== 0 ? "block" : "none",
            }}
          />
        </Tab>
      </Tabs>
      <CreateValue
        isOpen={isOpen}
        onOpenChange={onOpenChange}
        content={savedResponseValue}
      />
    </div>
  );
};

const MediaResponse: FC<{
  uri: string;
  contentType: string;
}> = ({ uri, contentType }) => {
  if (contentType.startsWith("image")) {
    return <img src={uri} alt={uri} />;
  }
  return <span>unsupport media</span>;
};

function getBodyLanguage(response: ResponseConnection): string {
  const contentTypeKey = Object.keys(response.headers).find(
    (x) => x.toLowerCase() === "content-type",
  );
  if (!contentTypeKey) return "text";

  const contentType = response.headers[contentTypeKey];
  if (!contentType) return "text";

  if (
    contentType.startsWith("application/json") ||
    contentType.startsWith("application/manifest+json")
  ) {
    return "json";
  }
  if (contentType.startsWith("text/html")) {
    return "html";
  }
  return "text";
}

function getBodyModel(response: ResponseConnection) {
  const language = getBodyLanguage(response);

  const jsonpBody = isJsonp(response.body);

  if (jsonpBody) {
    return {
      body: jsonpBody,
      language: "json",
    };
  }

  return {
    body: response.body,
    language,
  };
}

function asResponseValue(response: ResponseConnection, body: string): string {
  // version status
  // header1
  // header2
  // empty line(body parts separator)
  // body parts
  const versionStatus = `${response.version} ${response.status}`;
  const headers = Object.entries(response.headers)
    .map(([k, v]) => `${k}:${v}`)
    .join("\n");

  // biome-ignore lint: the format is more clearly.
  return versionStatus + "\n" + headers + "\n\n" + body;
}
