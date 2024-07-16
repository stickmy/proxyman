import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import toast from "react-hot-toast";
import { monaco } from "@/Monaco/Monaco";
import { getProcessorContent, setProcessor } from "@/Commands/Commands";
import { RuleMode } from "@/Events/ConnectionEvents";
import { ThemeType, useTheme } from "@/Components/TopBar/useTheme";
import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";

export const useRuleEditor = (
  elemId: string,
  packName: string,
  defaultValue?: string
) => {
  const [mode, setMode] = useState<RuleMode>(RuleMode.Redirect);
  const [currentPackName, setPackName] = useState<string>(packName);

  const serializationKey = useMemo(
    () => getRuleModeSerializationKey(mode),
    [mode]
  );

  const { theme, onThemeChange, offThemeChange } = useTheme();

  const setEditorTheme = (theme: ThemeType) => {
    if (editorRef.current) {
      editorRef.current.updateOptions({
        theme: theme === "light" ? "vs" : "vs-dark",
      });
    }
  };

  useEffect(() => {
    onThemeChange(setEditorTheme);

    return () => {
      offThemeChange(setEditorTheme);
    };
  }, []);

  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor>();
  const valueRef = useRef<string>();
  const saveRef = useRef<() => Promise<void>>();

  useEffect(() => {
    saveRef.current = async () => {
      if (!currentPackName) {
        return;
      }

      if (editorRef.current) {
        const value = editorRef.current.getValue();
        if (value === "") return;
        if (value === valueRef.current) return;

        try {
          await setProcessor(currentPackName, serializationKey, value);
          valueRef.current = value;
          toast.success("保存规则成功");
        } catch (error) {
          toast.error("保存规则失败");
        }
      }
    };
  }, [currentPackName, serializationKey]);

  const getContent = async () => {
    if (!currentPackName || !serializationKey) return undefined;

    try {
      return await getProcessorContent(currentPackName, serializationKey);
    } catch (error) {
      return undefined;
    }
  };

  useLayoutEffect(() => {
    setTimeout(() => {
      const node = document.getElementById(elemId);
      if (!node) return;

      (async () => {
        if (!editorRef.current) {
          const editor = createMonacoEditor(node, {
            theme,
            value: undefined,
            language: "shell",
            lineNumbers: "on",
          });
          editor.addAction({
            id: "save-processor-file",
            label: "save rule content",
            keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
            run: () => {
              saveRef.current?.();
            },
          });
          editorRef.current = editor;
        }

        const content = await getContent();
        valueRef.current = content;
        editorRef.current.setValue(content || defaultValue || "");
      })();
    }, 100);

    return () => {
      saveRef.current?.();
    };
  }, [serializationKey, currentPackName]);

  return {
    setMode,
    setPackName,
  };
};

const getRuleModeSerializationKey = (mode: RuleMode) => {
  if (mode === RuleMode.Delay) return "Delay";
  if (mode === RuleMode.Redirect) return "Redirect";
  throw new TypeError("Unsupported rule");
};
