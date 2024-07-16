import { monaco } from "@/Monaco/Monaco";
import { RefObject, useState } from "react";

export const usePretty = (
  editor: RefObject<monaco.editor.IStandaloneCodeEditor | undefined>,
  value: string
) => {
  const [isPretty, _setPrettyStatus] = useState<boolean>(false);

  const pretty = async () => {
    if (!editor.current) return;

    if (!isPretty) {
      editor.current.updateOptions({ readOnly: false });
      await editor.current.getAction("editor.action.formatDocument")?.run();
      editor.current.updateOptions({ readOnly: true });
    } else {
      editor.current.setValue(value);
    }

    _setPrettyStatus(!isPretty);
  };

  return {
    isPretty,
    pretty,
  };
};
