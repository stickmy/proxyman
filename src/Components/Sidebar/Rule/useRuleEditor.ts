import { useLayoutEffect, useRef } from "react";
import { monaco } from "@/Monaco/Monaco";
import { RuleEditorProps } from "@/Components/Sidebar/Rule/Rule";
import { getProcessorContent, setProcessor } from "@/Commands/Commands";
import { RuleMode } from "@/Events/ConnectionEvents";
import { ThemeType, useTheme } from "@/Components/TopBar/useTheme";

export const useRuleEditor = (
  elemId: string,
  mode: RuleMode,
  setSaveAction: RuleEditorProps["setSaveAction"],
  defaultValue?: string
) => {
  const { theme } = useTheme();

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
      const editor = createRuleEditor(node, theme);
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

const createRuleEditor = (elem: HTMLElement, theme: ThemeType) => {
  return monaco.editor.create(elem, {
    value: undefined,
    language: "shell",
    theme: transformSystemThemeToMonaco(theme),
    minimap: {
      enabled: false,
    },
    scrollbar: {
      verticalScrollbarSize: 4,
    },
  });
};

const transformSystemThemeToMonaco = (theme: ThemeType) => {
  return theme === "light" ? "vs" : "vs-dark";
};
