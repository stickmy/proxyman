import React, { FC, useEffect, useLayoutEffect, useRef, useState } from "react";
import cls from "classnames";
import dayjs from "dayjs";
import { ResponseConnection, RuleMode } from "@/Events/ConnectionEvents";
import { monaco } from "@/Monaco/Monaco";
import { isJsonp } from "@/Routes/Connections/Detail/Helper";
import toast from "react-hot-toast";
import { Tabs, Tab, Chip } from "@nextui-org/react";
import { removeResponseMapping, setProcessor } from "@/Commands/Commands";
import { usePretty } from "./Hooks/usePretty";
import { useConnActionStore } from "@/Routes/Connections/ConnActionStore";
import { Headers } from "@/Routes/Connections/Detail/Headers";
import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";
import { useTheme } from "@/Components/TopBar/useTheme";
import { usePackStore } from "@/Routes/Rule/usePacks";

export const Response: FC<{
  uri: string;
  response: ResponseConnection;
}> = ({ uri, response }) => {
  const { packs } = usePackStore();

  const { theme } = useTheme();

  const [activeTab, setActiveTab] = useState<string | number>("body");

  const resMonacoRef = useRef<monaco.editor.IStandaloneCodeEditor>();

  const { isPretty, pretty } = usePretty(resMonacoRef, response.body);

  const contentTypeKey = Object.keys(response.headers).find(
    (x) => x.toLowerCase() === "content-type"
  );

  useLayoutEffect(() => {
    if (activeTab !== "body") return;

    setTimeout(() => {
      const target = document.getElementById("res-body");

      if (target) {
        const { body, language } = getBodyModel(response);

        resMonacoRef.current = createMonacoEditor(target, {
          value: body,
          readonly: true,
          language,
          theme,
        });
      }
    }, 100);
  }, [activeTab]);

  const [beEditing, setBeEditing] = useState<boolean>(false);
  const onEditClick = () => {
    if (!resMonacoRef.current) return;

    resMonacoRef.current.updateOptions({
      readOnly: beEditing,
    });

    setBeEditing(!beEditing);
  };
  useEffect(() => {
    if (!beEditing) {
      // Response 规则的生命周期是 session 周期, 它的设置也不需要指定 pack, 因此选取任一开启的 pack 即可
      const pack = packs.find((x) => x.enable);

      if (!pack) {
        toast("请开启任一规则");
        return;
      }

      if (!resMonacoRef.current) return;

      const body = resMonacoRef.current.getValue();

      if (body === response.body) return;

      const rule = asProcessorRule(uri, response, body);

      (async () => {
        try {
          await setProcessor(pack.packName, RuleMode.Response, rule);
          setDetailVisible(false);
        } catch (error: any) {
          toast.error(error);
        }
      })();
    }
  }, [beEditing]);

  const { setDetailVisible } = useConnActionStore();

  const dropEditedResponse = async () => {
    const pack = packs.find((x) => x.enable);

    if (!pack) {
      toast("请开启任一规则");
      return;
    }

    try {
      await removeResponseMapping(pack.packName, uri);
      setDetailVisible(false);
    } catch (error: any) {
      toast.error(`重置失败`);
    }
  };

  const notStringLike = !!(
    contentTypeKey &&
    ["image", "octet-stream", "media"].some((type) =>
      response.headers[contentTypeKey].includes(type)
    )
  );

  return (
    <div className="flex flex-col h-full">
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
                "mr-1",
                "cursor-pointer",
                isPretty ? "bg-success-300" : "bg-default-100"
              )}
            >
              格式化
            </Chip>
            {/* <Button
              onClick={pretty}
              size="mini"
              type={isPretty ? "primary" : "default"}
              className="mr-1"
            >
              Pretty
            </Button>
            <Button
              size="mini"
              type={beEditing ? "primary" : "default"}
              onClick={onEditClick}
            >
              {beEditing ? "Save as next return" : "Edit"}
            </Button>
            {response.hitRules?.includes(RuleMode.Response) && (
              <Button
                size="mini"
                type="dashed"
                className="ml-1"
                onClick={dropEditedResponse}
              >
                Drop edited response
              </Button> */}
            {/* )} */}
          </div>
          {notStringLike && <div>"Media type not supported"</div>}
          <div
            id="res-body"
            className="w-full h-full relative border border-transparent data-[editing=true]:border-dashed data-[editing=true]:border-blue9"
            data-editing={beEditing}
            style={{
              display:
                !notStringLike && response.body.length !== 0 ? "block" : "none",
            }}
          ></div>
        </Tab>
      </Tabs>
    </div>
  );
};

function getBodyLanguage(response: ResponseConnection): string {
  const contentTypeKey = Object.keys(response.headers).find(
    (x) => x.toLowerCase() === "content-type"
  );
  if (!contentTypeKey) return "text";

  const contentType = response.headers[contentTypeKey];
  if (!contentType) return "text";

  if (
    contentType.startsWith("application/json") ||
    contentType.startsWith("application/manifest+json")
  ) {
    return "json";
  } else if (contentType.startsWith("text/html")) {
    return "html";
  } else {
    return "text";
  }
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

function asProcessorRule(
  uri: string,
  response: ResponseConnection,
  body: string
): string {
  // req pattern
  // version status
  // header1
  // header2
  // empty line(body parts separator)
  // body parts
  let versionStatus = `${response.version} ${response.status}`;
  let headers = Object.entries(response.headers)
    .map(([k, v]) => `${k}:${v}`)
    .join("\n");

  return uri + "\n" + versionStatus + "\n" + headers + "\n\n" + body;
}
