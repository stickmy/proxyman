import type { monaco } from "@/Monaco/Monaco";
import { type RefObject, useState } from "react";

export const useDecodeURIComponent = (
  editor: RefObject<monaco.editor.IStandaloneCodeEditor | undefined>,
  value: string,
) => {
  const [isDecoded, _setDecodeStatus] = useState<boolean>(false);

  const decode = async () => {
    if (!editor.current) return;

    if (!isDecoded) {
      editor.current.updateOptions({ readOnly: false });
      editor.current.setValue(decodeURIComponent(value));
      editor.current.updateOptions({ readOnly: true });
    } else {
      editor.current.setValue(value);
    }

    _setDecodeStatus(!isDecoded);
  };

  return {
    isDecoded,
    decode,
  };
};
