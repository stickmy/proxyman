import { Drawer } from "@/Components/Drawer/Drawer";
import { useLayout } from "@/Components/TopBar/useLayout";
import { SearchIcon } from "@/Icons";
import { useConnActionStore } from "@/Routes/Connections/ConnActionStore";
import { ConnectionDetail } from "@/Routes/Connections/Detail/ConnectionDetail";
import { Filters } from "@/Routes/Connections/Filters";
import { type Connection, useConnectionStore } from "@/Store/ConnectionStore";
import { usePinUriStore } from "@/Store/PinUriStore";
import {
  Button,
  Chip,
  Input,
  type Selection,
  Snippet,
  Spinner,
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
} from "@nextui-org/react";
import { useDebounceEffect } from "ahooks";
import dayjs from "dayjs";
import get from "lodash/get";
import { type FC, useCallback, useEffect, useState } from "react";
import { Pin } from "./Pin";
import { useNavigate } from "react-router-dom";
import "./index.css";

export const Connections = () => {
  const [layout] = useLayout();

  const navigate = useNavigate();
  const { connections } = useConnectionStore();
  const { filter } = useConnActionStore();
  const { currentPin } = usePinUriStore();

  const { detailVisible, setDetailVisible } = useConnActionStore();
  const [detailConn, setDetailConnection] = useState<Connection>();
  const [selectedKeys, setSelectedKeys] = useState<Selection>(new Set([]));

  const updateCurrentConn = (selection: Selection) => {
    setSelectedKeys(selection);

    if (selection === "all") return;

    const id: string = selection.keys().next().value;
    setDetailConnection(connections.find((x) => x.id === id));
  };

  const clearCurrentConn = () => {
    setSelectedKeys(new Set([]));
    setDetailConnection(undefined);
  };

  useEffect(() => {
    setDetailVisible(!!detailConn);
  }, [detailConn, setDetailVisible]);

  const [uriKeyword, setUriKeyword] = useState<string>();
  const [renderConnections, setRenderConnections] = useState<Connection[]>([]);

  useDebounceEffect(
    () => {
      // Ignore other searches when a pined uri exists.
      if (currentPin) {
        setRenderConnections(
          connections.filter((x) => x.request.uri.startsWith(currentPin)),
        );
        return;
      }

      let filtered = connections;

      if (uriKeyword) {
        filtered = connections.filter((x) =>
          x.request.uri.toLowerCase().includes(uriKeyword.toLowerCase()),
        );
      }
      if (filter) {
        filtered = filtered.filter((x) => filter(x));
      }

      setRenderConnections(filtered);
    },
    [uriKeyword, connections, currentPin, filter],
    { wait: 150 },
  );

  const renderCell = useCallback(
    (conn: Connection, columnKey: (typeof columns)[number]["key"] | number) => {
      switch (columnKey) {
        case "PIN": {
          const uri = conn.request.uri;
          return <Pin uri={uri} />;
        }
        case "response.status": {
          const value = get(conn, columnKey);
          return value ? <Status status={value} /> : <Spinner size="sm" />;
        }
        case "request.uri": {
          const value = get(conn, columnKey);

          return (
            <Snippet
              hideSymbol
              className="w-full"
              size="sm"
              classNames={{ pre: "whitespace-normal break-all" }}
            >
              {value}
            </Snippet>
          );
        }
        case "request.method": {
          return get(conn, columnKey);
        }
        case "response.version": {
          return get(conn, columnKey) || "-";
        }
        case "request.time": {
          const value = get(conn, columnKey);
          return value ? dayjs(value).format("HH:mm:ss:SSS") : "-";
        }
        case "response.time": {
          const value = get(conn, columnKey);
          return value ? dayjs(value).format("HH:mm:ss:SSS") : "-";
        }
        case "DURATION": {
          return conn.response?.time && conn.request?.time
            ? `${conn.response.time - conn.request.time}ms`
            : "-";
        }
        case "response.effects": {
          const value = get(conn, columnKey);
          if (!value) return "/";
          return (
            <div>
              {Object.entries(value).map(([packName, effects]) => (
                <div key={packName}>
                  <span className="inline-block">{packName}</span>
                  <ul>
                    {effects.map((effect) => (
                      <li key={effect.name} className="list-disc">
                        <Button
                          isIconOnly
                          disableRipple
                          disableAnimation
                          className="bg-transparent w-fit !text-tiny !outline-0 hover:text-primary-400"
                          onClick={(evt) => {
                            evt.nativeEvent.stopImmediatePropagation();
                            evt.stopPropagation();

                            navigate(`/pack/${packName}/${effect.name}`);
                          }}
                        >
                          {effect.name}
                        </Button>
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
          );
        }
        default: {
          return get(conn, columnKey);
        }
      }
    },
    [],
  );

  return (
    <div className="flex flex-col h-full">
      <div className="z-20 px-2 py-2 bt-2 mb-2 flex flex-col select-none bg-content1">
        <Input
          size="sm"
          classNames={{
            input: "text-tiny",
          }}
          className="w-[600px]"
          startContent={
            <div className="flex items-center pr-2">
              <SearchIcon />
            </div>
          }
          value={uriKeyword}
          onValueChange={setUriKeyword}
          placeholder="输入关键词进行过滤"
        />
        <Filters />
      </div>
      <Table
        aria-label="Connections table"
        style={{
          height: "calc(100% - 96px)",
          position: "relative",
        }}
        classNames={{
          base: "overflow-hidden",
        }}
        selectionMode="single"
        color="success"
        radius="sm"
        isHeaderSticky
        isCompact
        removeWrapper={false}
        selectedKeys={selectedKeys}
        onSelectionChange={updateCurrentConn}
      >
        <TableHeader columns={columns}>
          {(column) => (
            <TableColumn key={column.key} maxWidth={column.maxWidth}>
              {column.label}
            </TableColumn>
          )}
        </TableHeader>
        <TableBody items={renderConnections} emptyContent="Empty connections">
          {(item) => (
            <TableRow key={item.id}>
              {(columnKey) => (
                <TableCell className="text-tiny">
                  {renderCell(item, columnKey)}
                </TableCell>
              )}
            </TableRow>
          )}
        </TableBody>
      </Table>
      <Drawer
        data-selectable
        backdrop="transparent"
        placement={layout}
        className={
          layout === "bottom"
            ? "w-full min-w-full !mb-0 h-3/4 max-h-full"
            : "h-full min-h-full !mb-0 w-3/4 max-w-full"
        }
        radius="none"
        hideCloseButton
        isOpen={detailVisible}
        onOpenChange={() => setDetailVisible(false)}
        onClose={clearCurrentConn}
      >
        {detailConn && <ConnectionDetail connection={detailConn} />}
      </Drawer>
    </div>
  );
};

const Status: FC<{
  status: number;
}> = ({ status }) => {
  if (status >= 400) {
    return (
      <Chip color="danger" size="sm">
        {status}
      </Chip>
    );
  }

  if (status >= 300) {
    return (
      <Chip color="warning" size="sm">
        {status}
      </Chip>
    );
  }

  if (status >= 200) {
    return (
      <Chip color="success" size="sm">
        {status}
      </Chip>
    );
  }

  return (
    <Chip color="default" size="sm">
      {status}
    </Chip>
  );
};

const columns = [
  {
    key: "PIN",
    label: "PIN",
    width: 24,
  },
  {
    key: "response.status",
    label: "STATUS",
    width: 64,
  },
  {
    key: "request.uri",
    label: "URI",
    width: 400,
    maxWidth: 600,
  },
  {
    key: "request.method",
    label: "METHOD",
  },
  {
    key: "response.version",
    label: "VERSION",
  },
  {
    key: "request.time",
    label: "BEGIN TIME",
  },
  {
    key: "response.time",
    label: "END TIME",
  },
  {
    key: "DURATION",
    label: "DURATION",
  },
  {
    key: "response.effects",
    label: "HIT RULES",
  },
];
