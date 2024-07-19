import { Request } from "@/Routes/Connections/Detail/Request";
import { Response } from "@/Routes/Connections/Detail/Response";
import type { Connection } from "@/Store/ConnectionStore";
import { Spinner, Tab, Tabs } from "@nextui-org/react";
import React, { type FC } from "react";
import "./index.css";

export const ConnectionDetail: FC<{
  connection: Connection;
}> = ({ connection }) => {
  return (
    <Tabs
      aria-label="Connection detail"
      classNames={{
        tabList: "w-full",
      }}
    >
      <Tab key="request" title="Request" className="h-full">
        <Request request={connection.request} />
      </Tab>
      <Tab key="response" title="Response" className="h-full">
        {connection.response ? (
          <Response
            uri={connection.request.uri}
            response={connection.response}
          />
        ) : (
          <Spinner />
        )}
      </Tab>
    </Tabs>
  );
};
