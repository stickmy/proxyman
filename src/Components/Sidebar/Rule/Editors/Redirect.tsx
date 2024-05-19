import React, { FC } from "react";
import { RuleEditorProps } from "@/Components/Sidebar/Rule/Rule";
import { useRuleEditor } from "@/Components/Sidebar/Rule/useRuleEditor";
import { RuleMode } from "@/Events/ConnectionEvents";

export const Redirect: FC<RuleEditorProps> = ({ setSaveAction }) => {
  useRuleEditor(
    "redirect-editor",
    RuleMode.Redirect,
    setSaveAction,
    "# Example\n# https://uri.com/(.*) https://redirect.com/$1"
  );

  return <div id="redirect-editor" className="w-full h-full"></div>;
};
