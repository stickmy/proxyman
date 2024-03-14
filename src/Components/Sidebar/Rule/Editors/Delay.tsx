import React, { FC } from "react";
import { RuleEditorProps } from "@/Components/Sidebar/Rule/Rule";
import { useRuleEditor } from "@/Components/Sidebar/Rule/useRuleEditor";

import { RuleMode } from "@/Events/ConnectionEvents";

export const Delay: FC<RuleEditorProps> = ({ setSaveAction }) => {
  useRuleEditor(
    "delay-editor",
    RuleMode.Delay,
    setSaveAction,
    "# Example\n# https://uri.com 200"
  );

  return <div id="delay-editor" className="w-full h-full"></div>;
};
