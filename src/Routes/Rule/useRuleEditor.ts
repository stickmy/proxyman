import { getProcessorContent, setProcessor } from "@/Commands/Commands";
import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";
import { useEditorTheme } from "@/Components/MonacoEditor/useEditorTheme";
import { RuleMode } from "@/Events/ConnectionEvents";
import { monaco } from "@/Monaco/Monaco";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import toast from "react-hot-toast";

export const useRuleEditor = (
  elemId: string,
  packName: string,
  defaultValue?: string,
) => {
  const [mode, setMode] = useState<RuleMode>(RuleMode.Redirect);
  const [currentPackName, setPackName] = useState<string>(packName);

  const serializationKey = useMemo(
    () => getRuleModeSerializationKey(mode),
    [mode],
  );

  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor>();
  const valueRef = useRef<string>();
  const saveRef = useRef<() => Promise<void>>();

  const { theme } = useEditorTheme(editorRef);

  useEffect(() => {
    saveRef.current = async () => {
      if (!currentPackName) {
        return;
      }

      if (editorRef.current) {
        const value = editorRef.current.getValue();
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
            label: "save processor content",
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
  if (mode === RuleMode.Response) return "Response";
  throw new TypeError("Unsupported rule");
};
