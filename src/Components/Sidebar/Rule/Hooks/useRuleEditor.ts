import { useLayoutEffect, useRef } from "react";
import { monaco } from "@/Monaco/Monaco";
import { RuleEditorProps } from "@/Components/Sidebar/Rule/Rule";
import { getProcessorContent, setProcessor } from "@/Commands/Commands";
import { RuleMode } from "@/Events/ConnectionEvents";
import { useTheme } from "@/Components/TopBar/useTheme";
import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";
import { usePackStore } from "./usePacks";

export const useRuleEditor = (
  elemId: string,
  mode: RuleMode,
  setSaveAction: RuleEditorProps["setSaveAction"],
  defaultValue?: string
) => {
  const { currentPack } = usePackStore();

  const { theme } = useTheme();

  const serializationKey = getRuleModeSerializationKey(mode);

  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor>();
  const valueRef = useRef<string>();

  setSaveAction(async () => {
    if (!currentPack) {
      return;
    }

    if (editorRef.current) {
      const value = editorRef.current.getValue();

      if (value === valueRef.current) return;

      await setProcessor(currentPack, serializationKey, value);

      valueRef.current = value;
    }
  });

  useLayoutEffect(() => {
    setTimeout(() => {
      const node = document.getElementById(elemId);

      if (!node || editorRef.current) return;

      (async () => {
        const editor = createMonacoEditor(node, {
          theme,
          value: undefined,
          language: "shell",
          lineNumbers: "on",
        });
        editorRef.current = editor;

        try {
          const content = currentPack
            ? await getProcessorContent(currentPack, serializationKey)
            : undefined;
          if (content) {
            editor.setValue(content);
          } else if (defaultValue) {
            editor.setValue(defaultValue);
          }
        } catch (error: any) {
          // Notification.error({ content: "get rule content failed" });
        }
      })();
    }, 200);
  }, []);
};

const getRuleModeSerializationKey = (mode: RuleMode) => {
  if (mode === RuleMode.Delay) return "Delay";
  if (mode === RuleMode.Redirect) return "Redirect";
  throw new TypeError("Unsupported rule");
};
