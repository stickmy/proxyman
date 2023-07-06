import React, { FC, PropsWithChildren, useEffect, useState } from "react";
import {
  Button,
  Drawer,
  Input,
  Table,
  TableColumnProps,
  Tag,
  Tooltip,
} from "@arco-design/web-react";
import {
  IconBrush,
  IconLoading,
  IconPushpin,
} from "@arco-design/web-react/icon";
import { ConnectionDetail } from "@/Components/Connections/Detail/ConnectionDetail";
import { useDebounceEffect } from "ahooks";
import { Connection, useConnectionStore } from "@/Store/ConnectionStore";
import { usePinUriStore } from "@/Store/PinUriStore";
import dayjs from "dayjs";
import { useDetailVisible } from "@/Components/Connections/DetailVisibleStore";

export const Connections = () => {
  const { connections, clearConnections } = useConnectionStore();
  const { currentPin, pinUri, unpinUri, pins } = usePinUriStore();

  const { visible, setVisible } = useDetailVisible();
  const [detailConn, setDetailConn] = useState<Connection>();
  useEffect(() => {
    setVisible(!!detailConn);
  }, [detailConn]);

  const [uriKeyword, setUriKeyword] = useState<string>();
  const [renderConnections, setRenderConnections] = useState<Connection[]>([]);
  useDebounceEffect(
    () => {
      // Ignore uri keyword search when a pined uri exists.
      if (currentPin) return;

      if (!uriKeyword) {
        setRenderConnections(connections);
      } else {
        setRenderConnections(
          connections.filter((x) => x.request.uri.toLowerCase().includes(uriKeyword.toLowerCase()))
        );
      }
    },
    [uriKeyword, connections, currentPin],
    { wait: 150 }
  );
  useEffect(() => {
    if (currentPin) {
      setRenderConnections(
        connections.filter((x) => x.request.uri.startsWith(currentPin))
      );
    }
  }, [connections, currentPin]);

  const columns: TableColumnProps<Connection>[] = [
    {
      title: "pin",
      key: "pin",
      render: (_, item) =>
        pins.includes(item.request.uri) ? (
          <IconPushpin
            className="text-blue11 cursor-pointer"
            style={{ strokeWidth: 3 }}
            onClick={(evt) => {
              evt.stopPropagation();
              unpinUri(item.request.uri);
            }}
          />
        ) : (
          <IconPushpin
            className="cursor-pointer hover:text-blue11 hover:stroke-[3]"
            onClick={(evt) => {
              evt.stopPropagation();
              pinUri(item.request.uri);
            }}
          />
        ),
      width: 48,
    },
    {
      title: "status",
      key: "status",
      dataIndex: "response.status",
      render: (col) => (col ? <Status status={col} /> : <IconLoading />),
      width: 64,
    },
    {
      title: "method",
      key: "method",
      dataIndex: "request.method",
      width: 80,
    },
    {
      title: "version",
      key: "version",
      dataIndex: "response.version",
      render: (col) => col || "-",
      width: 120,
    },
    {
      title: "begin time",
      key: "begin-time",
      dataIndex: "request.time",
      render: (col) => (col ? dayjs(col).format("HH:mm:ss:SSS") : "-"),
      width: 120,
    },
    {
      title: "complete time",
      key: "complete-time",
      dataIndex: "response.time",
      render: (col) => (col ? dayjs(col).format("HH:mm:ss:SSS") : "-"),
      width: 140,
    },
    {
      title: "duration",
      key: "duration",
      dataIndex: "status",
      render: (_, item) =>
        item.response?.time && item.request?.time
          ? `${item.response.time - item.request.time}ms`
          : "-",
      width: 100,
    },
    {
      title: "hit rules",
      key: "rules",
      dataIndex: "hitRules",
      render: (_, item) =>
        item.response?.hitRules ? item.response.hitRules.join(", ") : "-",
      width: 200,
    },
    {
      title: "uri",
      key: "uri",
      dataIndex: "request.uri"
    },
  ];

  return (
    <>
      <div className="sticky top-0 z-20 px-1 py-1 mb-1 flex flex-row items-center shadow-md">
        <Input.Search
          size="small"
          value={uriKeyword}
          onChange={setUriKeyword}
          className="w-[300px] mr-2"
        />
        <div className="text-gray12 mr-2 text-sm w-[64px]">
          total {renderConnections.length}
        </div>
        <Tooltip mini content="clear">
          <Button
            icon={<IconBrush className="text-blue11" />}
            onClick={clearConnections}
          />
        </Tooltip>
      </div>
      <Table
        rowKey="id"
        onRow={(record) => ({
          onClick: () => setDetailConn(record),
        })}
        data={renderConnections}
        columns={columns}
        noDataElement={<span>Empty connections</span>}
        style={{
          height: "calc(100% - 46px)",
          overflowY: "scroll",
          position: "relative",
        }}
        size="mini"
        pagination={false}
        components={{
          header: {
            wrapper: StickyHeader,
          },
          // body: {
          //   row: ConnectionContextMenu,
          // },
        }}
      />
      <Drawer
        // className="bg-gray2"
        title={null}
        footer={null}
        width={800}
        visible={visible}
        closable={false}
        onCancel={() => setDetailConn(undefined)}
      >
        {detailConn && <ConnectionDetail connection={detailConn} />}
      </Drawer>
    </>
  );
};

const StickyHeader: FC<PropsWithChildren> = ({ children }) => {
  return (
    <div
      style={{
        position: "sticky",
        top: 0,
        zIndex: 5,
      }}
    >
      {children}
    </div>
  );
};

const Status: FC<{
  status: number;
}> = ({ status }) => {
  if (status >= 400)
    return (
      <Tag color="#f53f3f" size="small">
        {status}
      </Tag>
    );
  if (status >= 300)
    return (
      <Tag color="#ff7d00" size="small">
        {status}
      </Tag>
    );
  if (status >= 200)
    return (
      <Tag color="#7bc616" size="small">
        {status}
      </Tag>
    );
  return <span>{status}</span>;
};
