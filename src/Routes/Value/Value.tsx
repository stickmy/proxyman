import { getValueContent, saveValue } from "@/Commands/Commands";
import { createMonacoEditor } from "@/Components/MonacoEditor/MonacoEditor";
import { useEditorTheme } from "@/Components/MonacoEditor/useEditorTheme";
import { monaco } from "@/Monaco/Monaco";
import { type FC, useEffect, useLayoutEffect, useRef, useState } from "react";
import toast from "react-hot-toast";
import { useParams } from "react-router-dom";

export const Value: FC = () => {
  const { name } = useParams<{ name: string }>();

  const [content, setContent] = useState<string>();

  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor>();
  const valueRef = useRef<string>();
  const saveRef = useRef<() => Promise<void>>();
  const { theme } = useEditorTheme(editorRef);

  useEffect(() => {
    if (!name) return;

    getValueContent(name)
      .then(setContent)
      .catch((error) => {
        toast.error(`获取内容失败: ${error}`);
      });
  }, [name]);

  useEffect(() => {
    saveRef.current = async () => {
      if (!name) {
        return;
      }

      if (editorRef.current) {
        const value = editorRef.current.getValue();
        if (value === valueRef.current) return;

        try {
          await saveValue(name, value);
          valueRef.current = value;
          toast.success("保存成功");
        } catch (error) {
          toast.error("保存失败");
        }
      }
    };
  }, [name]);

  useLayoutEffect(() => {
    setTimeout(() => {
      const node = document.getElementById(VALUE_EDITOR_ELEM_ID);
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
            id: "save-value-file",
            label: "save value file content",
            keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
            run: () => {
              saveRef.current?.();
            },
          });
          editorRef.current = editor;
        }

        valueRef.current = content;
        editorRef.current.setValue(content || "");
      })();
    }, 100);

    return () => {
      saveRef.current?.();
    };
  }, [content]);

  if (!name) return null;

  return <div id={VALUE_EDITOR_ELEM_ID} className="w-full h-full" />;
};

const VALUE_EDITOR_ELEM_ID = "value-file-editor";
