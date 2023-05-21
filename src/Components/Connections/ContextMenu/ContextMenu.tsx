import React, { FC, PropsWithChildren } from "react";
import * as ContextMenu from "@radix-ui/react-context-menu";
import { CheckIcon } from "@radix-ui/react-icons";
import { Connection } from "@/Store/ConnectionStore";
import { usePinUriStore } from "@/Store/PinUriStore";
import { useRuleActions } from "@/Components/Sidebar/Rule/RuleActions";

import { RuleMode } from "@/Events/ConnectionEvents";
import { removeResponseMapping } from "@/Commands/Commands";
import { Notification } from "@arco-design/web-react";

export const ConnectionContextMenu: FC<
  PropsWithChildren<{
    record: Connection;
  }>
> = ({ children, ...rest }) => {
  const conn = rest.record;
  const hitRules = conn.response?.hitRules || [];

  const { editRule } = useRuleActions();
  const { pinUri, unpinUri, pins } = usePinUriStore();

  const pined = pins.includes(conn.request.uri);

  const onPinChange = () => {
    if (pined) {
      unpinUri(conn.request.uri);
    } else {
      pinUri(conn.request.uri);
    }
  };

  const onDelayChange = () => {
    editRule(RuleMode.Delay, conn.request.uri);
  };

  const onResponseChange = async (checked: boolean) => {
    if (!checked) {
      try {
        await removeResponseMapping(conn.request.uri);
      } catch (error: any) {
        Notification.error({
          content: "Cancel response mapping failed",
        });
      }
    }
  };

  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger asChild>
        <tr className="arco-table-tr" {...rest}>
          {children}
        </tr>
      </ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Content
          alignOffset={5}
          data-align="end"
          className="z-10 min-w-[220px] bg-white rounded-md overflow-hidden p-[5px] shadow-[0px_10px_38px_-10px_rgba(22,_23,_24,_0.35),_0px_10px_20px_-15px_rgba(22,_23,_24,_0.2)]"
        >
          <ContextMenu.CheckboxItem
            className="group text-[13px] leading-none text-violet11 rounded-[3px] flex items-center h-[25px] px-[5px] relative pl-[25px] select-none outline-none data-[disabled]:text-mauve8 data-[disabled]:pointer-events-none data-[highlighted]:bg-violet9 data-[highlighted]:text-violet1"
            checked={pined}
            onCheckedChange={onPinChange}
          >
            <ContextMenu.ItemIndicator className="absolute left-0 w-[25px] inline-flex items-center justify-center">
              <CheckIcon />
            </ContextMenu.ItemIndicator>
            Pin
          </ContextMenu.CheckboxItem>
          <ContextMenu.CheckboxItem
            className="group text-[13px] leading-none text-violet11 rounded-[3px] flex items-center h-[25px] px-[5px] relative pl-[25px] select-none outline-none data-[disabled]:text-mauve8 data-[disabled]:pointer-events-none data-[highlighted]:bg-violet9 data-[highlighted]:text-violet1"
            checked={hitRules.includes(RuleMode.Redirect)}
            // onCheckedChange={onDelayChange}
          >
            <ContextMenu.ItemIndicator className="absolute left-0 w-[25px] inline-flex items-center justify-center">
              <CheckIcon />
            </ContextMenu.ItemIndicator>
            Redirect
          </ContextMenu.CheckboxItem>
          <ContextMenu.CheckboxItem
            className="group text-[13px] leading-none text-violet11 rounded-[3px] flex items-center h-[25px] px-[5px] relative pl-[25px] select-none outline-none data-[disabled]:text-mauve8 data-[disabled]:pointer-events-none data-[highlighted]:bg-violet9 data-[highlighted]:text-violet1"
            checked={hitRules.includes(RuleMode.Delay)}
            onCheckedChange={onDelayChange}
          >
            <ContextMenu.ItemIndicator className="absolute left-0 w-[25px] inline-flex items-center justify-center">
              <CheckIcon />
            </ContextMenu.ItemIndicator>
            Delay
          </ContextMenu.CheckboxItem>
          <ContextMenu.CheckboxItem
            className="group text-[13px] leading-none text-violet11 rounded-[3px] flex items-center h-[25px] px-[5px] relative pl-[25px] select-none outline-none data-[disabled]:text-mauve8 data-[disabled]:pointer-events-none data-[highlighted]:bg-violet9 data-[highlighted]:text-violet1"
            checked={hitRules.includes(RuleMode.Response)}
            onCheckedChange={onResponseChange}
          >
            <ContextMenu.ItemIndicator className="absolute left-0 w-[25px] inline-flex items-center justify-center">
              <CheckIcon />
            </ContextMenu.ItemIndicator>
            Modified response
          </ContextMenu.CheckboxItem>
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
};
