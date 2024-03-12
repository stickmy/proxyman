import { Divider, Space, Tag } from "@arco-design/web-react";
import { Connection } from "@/Store/ConnectionStore";
import { useConnActionStore } from "@/Components/Connections/ConnActionStore";

interface Filter {
  label: string;
  filter: (conn: Connection) => boolean;
}

const status: Array<Filter> = ["100", "200", "300", "400", "500"].map(
  (status) => ({
    label: status,
    filter: (conn) =>
      conn.response
        ? conn.response.status.toString().startsWith(status[0])
        : false,
  })
);

const versions: Array<Filter> = [
  {
    label: "HTTP1",
    filter: (conn) =>
      conn.response
        ? conn.response.version.toLowerCase().startsWith("http/1")
        : false,
  },
  {
    label: "HTTP2",
    filter: (conn) =>
      conn.response
        ? conn.response.version.toLowerCase().startsWith("http/2")
        : false,
  },
];

const responseTypes: Array<Filter> = [
  {
    label: "JSON",
    filter: (conn) =>
      conn.response
        ? conn.response.headers["content-type"]?.toLowerCase().includes("json")
        : false,
  },
  {
    label: "XML",
    filter: (conn) =>
      conn.response
        ? conn.response.headers["content-type"]?.toLowerCase().includes("xml")
        : false,
  },
  {
    label: "TEXT",
    filter: (conn) =>
      conn.response
        ? conn.response.headers["content-type"]?.toLowerCase() === "text/plain"
        : false,
  },
  {
    label: "HTML",
    filter: (conn) =>
      conn.response
        ? conn.response.headers["content-type"]?.toLowerCase().includes("html")
        : false,
  },
  {
    label: "JS",
    filter: (conn) =>
      conn.response
        ? conn.response.headers["content-type"]
            ?.toLowerCase()
            .includes("javascript")
        : false,
  },
  {
    label: "CSS",
    filter: (conn) =>
      conn.response
        ? conn.response.headers["content-type"]?.toLowerCase().includes("css")
        : false,
  },
  {
    label: "IMAGE",
    filter: (conn) =>
      conn.response
        ? conn.response.headers["content-type"]?.toLowerCase().includes("image")
        : false,
  },
  {
    label: "BINARY",
    filter: (conn) =>
      conn.response
        ? conn.response.headers["content-type"]
            ?.toLowerCase()
            .includes("octet-stream")
        : false,
  },
];

const filterAll = () => true;

export const Filters = () => {
  const { setFilter, filter } = useConnActionStore();

  return (
    <div className="mt-2 mb-2 conn-filters">
      <Space size={4}>
        <Tag
          className="filter cursor-pointer hover:bg-gray-200"
          data-active={filter === filterAll}
          onClick={() =>
            filter === filterAll ? setFilter(undefined) : setFilter(filterAll)
          }
        >
          All
        </Tag>
        <Divider type="vertical" />
        {versions.map((x) => (
          <Tag
            key={x.label}
            data-active={filter === x.filter}
            className="filter cursor-pointer hover:bg-gray-200"
            onClick={() => {
              filter === x.filter ? setFilter(undefined) : setFilter(x.filter);
            }}
          >
            {x.label}
          </Tag>
        ))}
        <Divider type="vertical" />
        {status.map((x) => (
          <Tag
            key={x.label}
            data-active={filter === x.filter}
            className="filter cursor-pointer hover:bg-gray-200"
            onClick={() => {
              filter === x.filter ? setFilter(undefined) : setFilter(x.filter);
            }}
          >
            {x.label}
          </Tag>
        ))}
        <Divider type="vertical" />
        {responseTypes.map((x) => (
          <Tag
            key={x.label}
            data-active={filter === x.filter}
            className="filter cursor-pointer hover:bg-gray-200"
            onClick={() => {
              filter === x.filter ? setFilter(undefined) : setFilter(x.filter);
            }}
          >
            {x.label}
          </Tag>
        ))}
      </Space>
    </div>
  );
};
