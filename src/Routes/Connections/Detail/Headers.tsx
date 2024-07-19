import { CodeIcon, TableIcon } from "@/Icons";
import {
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
  getKeyValue,
} from "@nextui-org/react";
import cls from "classnames";
import React, { type FC, useState } from "react";

export const Headers: FC<{
  headers: Record<string, string>;
}> = ({ headers }) => {
  const [showTable, setShowTable] = useState(true);

  return (
    <div className="headers scrollbar-hide">
      <div className="mb-3 headers-tools flex flex-row items-center bg-content1 z-10">
        <TableIcon
          className={cls("mr-1 cursor-pointer", { active: showTable })}
          onClick={() => setShowTable(true)}
        />
        <CodeIcon
          className={cls("cursor-pointer", { active: !showTable })}
          onClick={() => setShowTable(false)}
        />
      </div>
      {showTable ? (
        <Table
          aria-label="Connection headers"
          hideHeader
          removeWrapper
          fullWidth={false}
          layout="fixed"
        >
          <TableHeader columns={columns}>
            {column => (
              <TableColumn key={column.key}>{column.label}</TableColumn>
            )}
          </TableHeader>
          <TableBody
            items={Object.keys(headers).map(key => ({
              name: key,
              value: headers[key],
            }))}
          >
            {item => (
              <TableRow key={item.name}>
                {columnKey => (
                  <TableCell
                    className={cls(
                      "text-tiny border-1 border-default-200 border-solid rtl:first:border-r-0 break-all",
                      columnKey === "name" ? "w-[180px]" : "",
                    )}
                  >
                    {getKeyValue(item, columnKey)}
                  </TableCell>
                )}
              </TableRow>
            )}
          </TableBody>
        </Table>
      ) : (
        Object.keys(headers).map(key => (
          <div className="text-tiny mb-2" key={key}>
            <span className="inline-block mr-2 label">{key}</span>
            <span className="value break-all">{headers[key]}</span>
          </div>
        ))
      )}
    </div>
  );
};

const columns = [
  {
    key: "name",
    label: "NAME",
  },
  {
    key: "value",
    label: "VALUE",
  },
];
