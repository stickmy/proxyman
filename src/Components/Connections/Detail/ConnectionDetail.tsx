import React, { FC } from "react";
import { StickyContainer, Sticky } from "react-sticky";
import { Spin, Tabs } from "@arco-design/web-react";
import { Connection } from "@/Store/ConnectionStore";
import { Request } from "@/Components/Connections/Detail/Request";
import { Response } from "@/Components/Connections/Detail/Response";
import "./index.css";

export const ConnectionDetail: FC<{
  connection: Connection;
}> = ({ connection }) => {
  return (
    <StickyContainer>
      <Tabs
        type="text"
        defaultActiveTab="response"
        className="detail-panels"
        renderTabHeader={(props, DefaultTabHeader) => (
          <Sticky topOffset={-12}>
            {({ style, isSticky }) => (
              <DefaultTabHeader
                {...props}
                style={{
                  ...style,
                  top: isSticky ? 12 : 0,
                }}
              />
            )}
          </Sticky>
        )}
      >
        <Tabs.TabPane key="request" title="Request">
          <Request request={connection.request} />
        </Tabs.TabPane>
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
      </Tabs>
    </StickyContainer>
  );
};
