import React, { FC, PropsWithChildren, useEffect, useState } from "react";
import {
  Drawer,
  Empty,
  Input,
  Message,
  Table,
  TableColumnProps,
  Tag,
} from "@arco-design/web-react";
import {
  IconLoading,
  IconPushpin,
  IconSearch,
  IconCopy,
} from "@arco-design/web-react/icon";
import { ConnectionDetail } from "@/Components/Connections/Detail/ConnectionDetail";
import { useDebounceEffect } from "ahooks";
import { Connection, useConnectionStore } from "@/Store/ConnectionStore";
import { usePinUriStore } from "@/Store/PinUriStore";
import dayjs from "dayjs";
import { Filters } from "@/Components/Connections/Filters";
import { useConnActionStore } from "@/Components/Connections/ConnActionStore";
import "./connections.css";

export const Connections = () => {
  const { connections } = useConnectionStore();
  const { filter } = useConnActionStore();
  const { currentPin, pinUri, unpinUri, pins } = usePinUriStore();

  const { detailVisible, setDetailVisible } = useConnActionStore();
  const [detailConn, setDetailConnection] = useState<Connection>();
  useEffect(() => {
    setDetailVisible(!!detailConn);
  }, [detailConn]);

  const [uriKeyword, setUriKeyword] = useState<string>();
  const [renderConnections, setRenderConnections] = useState<Connection[]>([]);

  useDebounceEffect(
    () => {
      // Ignore other searches when a pined uri exists.
      if (currentPin) {
        setRenderConnections(
          connections.filter((x) => x.request.uri.startsWith(currentPin))
        );
        return;
      }

      let filtered = connections;

      if (uriKeyword) {
        filtered = connections.filter((x) =>
          x.request.uri.toLowerCase().includes(uriKeyword.toLowerCase())
        );
      }
      if (filter) {
        filtered = filtered.filter((x) => filter(x));
      }

      setRenderConnections(filtered);
    },
    [uriKeyword, connections, currentPin, filter],
    { wait: 150 }
  );

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
      title: "uri",
      key: "uri",
      dataIndex: "request.uri",
      render: (col) => (
        <span className="conn-uri">
          {col}
          <IconCopy
            className="uri-copy"
            onClick={(evt) => {
              evt.stopPropagation();
              navigator.clipboard.writeText(col).then(
                () => {
                  Message.success("URI 已复制");
                },
                (reason) => {
                  Message.warning("URI 复制失败");
                }
              );
            }}
          />
        </span>
      ),
    },

    {
      title: "method",
      key: "method",
      dataIndex: "request.method",
      width: 100,
    },
    {
      title: "version",
      key: "version",
      dataIndex: "response.version",
      render: (col) => col || "-",
      width: 120,
    },
    {
      title: "开始时间",
      key: "begin-time",
      dataIndex: "request.time",
      render: (col) => (col ? dayjs(col).format("HH:mm:ss:SSS") : "-"),
      width: 120,
    },
    {
      title: "完成时间",
      key: "complete-time",
      dataIndex: "response.time",
      render: (col) => (col ? dayjs(col).format("HH:mm:ss:SSS") : "-"),
      width: 140,
    },
    {
      title: "耗时",
      key: "duration",
      dataIndex: "status",
      render: (_, item) =>
        item.response?.time && item.request?.time
          ? `${item.response.time - item.request.time}ms`
          : "-",
      width: 100,
    },
    {
      title: "命中的规则",
      key: "rules",
      dataIndex: "hitRules",
      render: (_, item) =>
        item.response?.hitRules ? item.response.hitRules.join(", ") : "-",
      width: 160,
    },
  ];

  return (
    <>
      <div className="z-20 px-2 py-2 mb-2 flex flex-col select-none connections">
        <Input
          height={24}
          prefix={<IconSearch />}
          value={uriKeyword}
          onChange={setUriKeyword}
          className="w-[476px]"
          placeholder="输入关键词进行过滤"
        />
        <Filters />
      </div>
      <Table
        rowKey="id"
        onRow={(record) => ({
          onClick: () => setDetailConnection(record),
        })}
        data={renderConnections}
        columns={columns}
        noDataElement={<Empty description="等待请求" />}
        border={false}
        style={{
          height: "calc(100% - 88px)",
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
        data-selectable
        title={null}
        footer={null}
        width={"60vw"}
        wrapClassName="conn-detail-drawer"
        visible={detailVisible}
        closable={false}
        onCancel={() => setDetailConnection(undefined)}
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
        // position: "sticky",
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
  if (status >= 400) {
    return (
      <Tag color="#f53f3f" size="small">
        {status}
      </Tag>
    );
  }

  if (status >= 300) {
    return (
      <Tag color="#ff7d00" size="small">
        {status}
      </Tag>
    );
  }

  if (status >= 200) {
    return (
      <Tag color="#7bc616" size="small">
        {status}
      </Tag>
    );
  }

  return (
    <Tag color="#f1f1f1" size="small">
      {status}
    </Tag>
  );
};
