import React, { FC, useEffect, useLayoutEffect, useRef, useState } from "react";
import { ResponseConnection, RuleMode } from "@/Events/ConnectionEvents";
import { monaco } from "@/Monaco/Monaco";
import { isJsonp } from "@/Components/Connections/Detail/Helper";
import { Button, Notification } from "@arco-design/web-react";
import { LabelSeparator } from "@/Components/Connections/Detail/Separator";
import { removeResponseMapping, setProcessor } from "@/Commands/Commands";
import { usePretty } from "./Hooks/usePretty";
import dayjs from "dayjs";
import { useConnActionStore } from "@/Components/Connections/ConnActionStore";
import { Headers } from "@/Components/Connections/Detail/Headers";

export const Response: FC<{
  uri: string;
  response: ResponseConnection;
}> = ({ uri, response }) => {
  const monacoRef = useRef<monaco.editor.IStandaloneCodeEditor>();

  const { isPretty, pretty } = usePretty(monacoRef, response.body);

  const contentTypeKey = Object.keys(response.headers).find(
    (x) => x.toLowerCase() === "content-type"
  );

  useLayoutEffect(() => {
    setTimeout(() => {
      const target = document.getElementById("res-body");

      if (target) {
        const { body, language } = getBodyModel(response);

        monacoRef.current = monaco.editor.create(target, {
          value: body,
          language: language,
          lineNumbers: "off",
          inlayHints: {
            enabled: "off",
          },
          minimap: {
            enabled: false,
          },
          scrollbar: {
            verticalScrollbarSize: 4,
          },
          readOnly: true,
        });
      }
    }, 200);
  }, []);

  const [beEditing, setBeEditing] = useState<boolean>(false);
  const onEditClick = () => {
    if (!monacoRef.current) return;

    monacoRef.current.updateOptions({
      readOnly: beEditing,
    });

    setBeEditing(!beEditing);
  };
  useEffect(() => {
    if (!beEditing) {
      if (!monacoRef.current) return;

      const body = monacoRef.current.getValue();

      if (body === response.body) return;

      const rule = asProcessorRule(uri, response, body);

      (async () => {
        try {
          await setProcessor(RuleMode.Response, rule);
          setDetailVisible(false);
        } catch (error: any) {
          Notification.error({
            content: error,
          });
        }
      })();
    }
  }, [beEditing]);

  const { setDetailVisible } = useConnActionStore();

  const dropEditedResponse = async () => {
    try {
      await removeResponseMapping(uri);
      setDetailVisible(false);
    } catch (error: any) {
      Notification.error({
        content: "Drop edited response failed",
      });
    }
  };

  const notStringLike = !!(
    contentTypeKey &&
    ["image", "octet-stream", "media"].some((type) =>
      response.headers[contentTypeKey].includes(type)
    )
  );

  return (
    <div>
      <div className="item">
        <span className="inline-block w-[60px] label">version</span>
        <span className="value">{response.version}</span>
      </div>
      <div className="item">
        <span className="inline-block w-[60px] label">status</span>
        <span className="value">{response.status}</span>
      </div>
      <div className="item">
        <span className="inline-block w-[60px] label">time</span>
        <span className="value">
          {dayjs(response.time).format("YYYY-MM-DD HH:mm:ss:SSS")}
        </span>
      </div>
      <LabelSeparator label="Headers" />
      <Headers headers={response.headers} />
      <LabelSeparator label="Body" />
      <div className="mb-[10px]">
        <Button
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
          </Button>
        )}
      </div>
      {notStringLike
        ? "Media type not supported"
        : response.body.length !== 0 && (
            <div
              id="res-body"
              className="w-full h-[500px] relative border border-gray4 data-[editing=true]:border-dashed data-[editing=true]:border-blue9"
              data-editing={beEditing}
            ></div>
          )}
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
