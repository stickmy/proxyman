import { useLayoutEffect, useRef } from "react";
import { monaco } from "@/Monaco/Monaco";
import { RuleEditorProps } from "@/Components/Sidebar/Rule/Rule";
import { getProcessorContent, setProcessor } from "@/Commands/Commands";
import { RuleMode } from "@/Events/ConnectionEvents";
import { Notification } from "@arco-design/web-react";

export const useRuleEditor = (
  elemId: string,
  mode: RuleMode,
  setSaveAction: RuleEditorProps["setSaveAction"],
  defaultValue?: string
) => {
  const serializationKey = getRuleModeSerializationKey(mode);

  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor>();
  const valueRef = useRef<string>();

  setSaveAction(async () => {
    if (editorRef.current) {
      const value = editorRef.current.getValue();

      if (value === valueRef.current) return;

      await setProcessor(serializationKey, value);

      valueRef.current = value;
    }
  });

  useLayoutEffect(() => {
    const node = document.getElementById(elemId);

    if (!node || editorRef.current) return;

    (async () => {
      const editor = createRuleEditor(node, RuleMode.Delay);
      editorRef.current = editor;

      try {
        const content = await getProcessorContent(serializationKey);
        if (content) {
          editor.setValue(content);
        } else if (defaultValue) {
          editor.setValue(defaultValue);
        }
      } catch (error: any) {
        // Notification.error({ content: "get rule content failed" });
      }
    })();
  }, []);
};

const getRuleModeSerializationKey = (mode: RuleMode) => {
  if (mode === RuleMode.Delay) return "Delay";
  if (mode === RuleMode.Redirect) return "Redirect";
  throw new TypeError("Unsupported rule");
};

const createRuleEditor = (elem: HTMLElement, mode: RuleMode) => {
  return monaco.editor.create(elem, {
    value: undefined,
    language: "shell",
    minimap: {
      enabled: false,
    },
    scrollbar: {
      verticalScrollbarSize: 4,
    },
  });
};
