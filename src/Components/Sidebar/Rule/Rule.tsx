import React, { useRef, useState } from "react";
import { Drawer, Collapse, Switch, Message } from "@arco-design/web-react";
import { Redirect } from "@/Components/Sidebar/Rule/Editors/Redirect";
import { Delay } from "@/Components/Sidebar/Rule/Editors/Delay";
import { RuleMode } from "@/Events/ConnectionEvents";
import { usePackStore } from "./Hooks/usePacks";
import { updateProcessPackStatus } from "@/Commands/Commands";
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

  const packStore = usePackStore();

  const openRuleEditor = (mode: RuleMode, packName: string) => {
    packStore.setCurrentPack(packName);
    setOpenStatus(true);
    setRuleMode(mode);
  };

  const editorSaveHandler = useRef<() => void>();

  const onClose = async () => {
    setOpenStatus(false);

    if (editorSaveHandler.current) {
      try {
        await editorSaveHandler.current();
      } catch (error: any) {
        Message.error(`保存失败: ${error}`)
      }
    }
  };

  const setSaveAction = (handler: () => void) => {
    editorSaveHandler.current = handler;
  };

  const updatePackStatus = async (packName: string, enable: boolean) => {
    try {
      const ret = await updateProcessPackStatus(packName, enable);
      packStore.updatePackStatus(packName, enable);
    } catch (error) {
      Message.error(`${enable ? "开启" : "关闭"}规则失败`);
    }
  };

  return (
    <>
      <Collapse lazyload={false} bordered={false}>
        {packStore.packs.map((pack) => (
          <Collapse.Item
            name={pack.packName}
            key={pack.packName}
            header={<span>{pack.packName}</span>}
            extra={
              <Switch
                size="small"
                checked={pack.enable}
                onChange={(enable) => updatePackStatus(pack.packName, enable)}
              />
            }
          >
            <ul className="sidebar-item">
              {rules.map(({ label, mode }) => (
                <li
                  key={mode}
                  data-active={mode === ruleMode}
                  className="transition cursor-pointer"
                  onClick={() => openRuleEditor(mode, pack.packName)}
                >
                  {label}
                </li>
              ))}
            </ul>
          </Collapse.Item>
        ))}
      </Collapse>
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
