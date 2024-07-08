import React, { FC } from "react";
import { Collapse, Tooltip } from "@arco-design/web-react";
import { Pin } from "@/Components/Sidebar/Pin/Pin";
import {
  IconCodeBlock,
  IconInfoCircle,
  IconPushpin,
} from "@arco-design/web-react/icon";
import { Rule } from "@/Components/Sidebar/Rule/Rule";
import { RuleHeader } from "./Rule/RuleHeader";
import { Title } from "./Title/Title";
import "./sidebar.css";

export const Sidebar: FC = () => {
  return (
    <aside className="w-full aside">
      <Title
        tip={
          <Tooltip mini content="点击请求列表中任意一条进行过滤，再次点击取消">
            <IconInfoCircle />
          </Tooltip>
        }
      >
        <div>
          <IconPushpin className="mr-1" />
          Pin
        </div>
      </Title>
      <Pin />
      <Title>
        <RuleHeader />
      </Title>
      <Rule />
    </aside>
  );
};
