import React, { FC } from "react";
import { Spin, Tabs } from "@arco-design/web-react";
import { Connection } from "@/Store/ConnectionStore";
import { Request } from "@/Components/Connections/Detail/Request";
import { Response } from "@/Components/Connections/Detail/Response";
import "./index.css";

export const ConnectionDetail: FC<{
  connection: Connection;
}> = ({ connection }) => {
  return (
    <Tabs type="text">
      <Tabs.TabPane key="response" title="Response">
        {connection.response ? (
          <Response
            uri={connection.request.uri}
            response={connection.response}
          />
        ) : (
          <Spin />
        )}
      </Tabs.TabPane>
      <Tabs.TabPane key="request" title="Request">
        <Request request={connection.request} />
      </Tabs.TabPane>
    </Tabs>
  );
};
