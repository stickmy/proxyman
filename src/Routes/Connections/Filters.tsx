import { Chip, Divider } from "@nextui-org/react";
import { Connection } from "@/Store/ConnectionStore";
import { useConnActionStore } from "@/Routes/Connections/ConnActionStore";
import { FilterIcon } from "@/Icons/FilterIcon";

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
    <div className="mt-4 conn-filters flex items-center whitespace-nowrap space-x-2 h-6 scrollbar-hide overflow-x-scroll overflow-y-hidden">
      <FilterIcon className="flex-shrink-0 mr-2" size={18} />
      <Chip
        variant="dot"
        color="warning"
        size="sm"
        className="filter cursor-pointer"
        data-active={filter === filterAll}
        onClick={() =>
          filter === filterAll ? setFilter(undefined) : setFilter(filterAll)
        }
      >
        All
      </Chip>
      <Divider orientation="vertical" />
      {versions.map((x) => (
        <Chip
          key={x.label}
          variant="flat"
          size="sm"
          data-active={filter === x.filter}
          className="filter cursor-pointer"
          onClick={() => {
            filter === x.filter ? setFilter(undefined) : setFilter(x.filter);
          }}
        >
          {x.label}
        </Chip>
      ))}
      <Divider orientation="vertical" />
      {status.map((x) => (
        <Chip
          key={x.label}
          variant="flat"
          size="sm"
          data-active={filter === x.filter}
          className="filter cursor-pointer"
          onClick={() => {
            filter === x.filter ? setFilter(undefined) : setFilter(x.filter);
          }}
        >
          {x.label}
        </Chip>
      ))}
      <Divider orientation="vertical" />
      {responseTypes.map((x) => (
        <Chip
          key={x.label}
          variant="flat"
          size="sm"
          data-active={filter === x.filter}
          className="filter cursor-pointer"
          onClick={() => {
            filter === x.filter ? setFilter(undefined) : setFilter(x.filter);
          }}
        >
          {x.label}
        </Chip>
      ))}
    </div>
  );
};
