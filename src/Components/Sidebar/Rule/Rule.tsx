import React, { useRef, useState } from "react";
import { Drawer } from "@arco-design/web-react";
import { Redirect } from "@/Components/Sidebar/Rule/Editors/Redirect";
import { Delay } from "@/Components/Sidebar/Rule/Editors/Delay";
import { RuleMode } from "@/Events/ConnectionEvents";
import "./rule.css";

const rules = [
  {
    label: "重定向",
    mode: RuleMode.Redirect,
  },
  {
    label: "延时",
    mode: RuleMode.Delay,
  },
];

export const Rule = () => {
  const [openStatus, setOpenStatus] = useState<boolean>(false);
  const [ruleMode, setRuleMode] = useState<RuleMode>();

  const openRuleEditor = (mode: RuleMode) => {
    setOpenStatus(true);
    setRuleMode(mode);
  };

  const editorSaveHandler = useRef<() => void>();

  const onClose = async () => {
    setOpenStatus(false);

    if (editorSaveHandler.current) {
      try {
        await editorSaveHandler.current();
      } catch (error: any) {}
    }
  };

  const setSaveAction = (handler: () => void) => {
    editorSaveHandler.current = handler;
  };

  return (
    <>
      <ul className="sidebar-item">
        {rules.map(({ label, mode }) => (
          <li
            key={mode}
            data-active={mode === ruleMode}
            className="hover:text-gray-800 transition cursor-pointer"
            onClick={() => openRuleEditor(mode)}
          >
            {label}
          </li>
        ))}
      </ul>
      <Drawer
        title={null}
        footer={null}
        closable={false}
        width="80vw"
        visible={openStatus}
        onCancel={onClose}
      >
        {ruleMode === RuleMode.Redirect && (
          <Redirect setSaveAction={setSaveAction} />
        )}
        {ruleMode === RuleMode.Delay && <Delay setSaveAction={setSaveAction} />}
      </Drawer>
    </>
  );
};

export interface RuleEditorProps {
  setSaveAction: (action: () => Promise<void>) => void;
}
