import React, { FC, useState } from "react";
import cls from "classnames";
import {
  getKeyValue,
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
} from "@nextui-org/react";
import { CodeIcon, TableIcon } from "@/Icons";

export const Headers: FC<{
  headers: Record<string, string>;
}> = ({ headers }) => {
  const [showTable, setShowTable] = useState(true);

  return (
    <div className="headers scrollbar-hide">
      <div className="mb-3 headers-tools flex flex-row items-center">
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
        <Table aria-label="Connection headers" hideHeader removeWrapper>
          <TableHeader columns={columns}>
            {(column) => (
              <TableColumn key={column.key}>{column.label}</TableColumn>
            )}
          </TableHeader>
          <TableBody
            items={Object.keys(headers).map((key) => ({
              name: key,
              value: headers[key],
            }))}
          >
            {(item) => (
              <TableRow key={item.name}>
                {(columnKey) => (
                  <TableCell
                    className={cls(
                      "text-tiny border-1 border-default-400 border-solid rtl:first:border-r-0",
                      columnKey === "name" ? "w-[300px]" : ""
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
        Object.keys(headers).map((key) => (
          <div className="item" key={key}>
            <span className="inline-block mr-2 label">{key}</span>
            <span className="value">{headers[key]}</span>
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
