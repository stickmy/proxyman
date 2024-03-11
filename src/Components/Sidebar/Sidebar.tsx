import React, { FC } from "react";
import { Collapse, Tooltip } from "@arco-design/web-react";
import { Toolbar } from "./Toolbar/Toolbar";
import { Pin } from "@/Components/Sidebar/Pin/Pin";
import {
  IconCodeBlock,
  IconInfoCircle,
  IconPushpin,
} from "@arco-design/web-react/icon";
import { Rule } from "@/Components/Sidebar/Rule/Rule";

export const Sidebar: FC = () => {
  return (
    <aside className="w-full aside">
      <Toolbar />
      <Collapse
        lazyload={false}
        bordered={false}
        defaultActiveKey={["pin"]}
        className="bg-gray2 min-w-min"
      >
        <Collapse.Item
          name="pin"
          header={
            <div>
              <IconPushpin className="mr-1" />
              Pin
            </div>
          }
          className="bg-gray2 border-none"
          extra={
            <Tooltip mini content="点击请求列表中任意一条进行过滤，再次点击取消">
              <IconInfoCircle />
            </Tooltip>
          }
        >
          <Pin />
        </Collapse.Item>
        <Collapse.Item
          name="rule"
          className="bg-gray2 border-none"
          header={
            <span>
              <IconCodeBlock className="mr-1" />
              规则
            </span>
          }
        >
          <Rule />
        </Collapse.Item>
      </Collapse>
    </aside>
  );
};
