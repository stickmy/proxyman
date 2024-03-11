import React, { FC, useState } from "react";
import cls from "classnames";
import { IconCodeBlock, IconNav } from "@arco-design/web-react/icon";
import { Table } from "@arco-design/web-react";

export const Headers: FC<{
  headers: Record<string, string>;
}> = ({ headers }) => {
  const [showTable, setShowTable] = useState(true);

  return (
    <div className="headers">
      <div className="mb-1">
        <IconNav
          className={cls("header-pretty", { active: showTable })}
          onClick={() => setShowTable(true)}
        />
        <IconCodeBlock
          className={cls("header-pretty", { active: !showTable })}
          onClick={() => setShowTable(false)}
        />
      </div>
      {showTable ? (
        <Table
          pagination={false}
          border={{
            wrapper: true,
            cell: true,
          }}
          columns={[
            {
              key: "name",
              title: "name",
              dataIndex: "name",
              width: 200,
              headerCellStyle: {
                display: "none",
              },
            },
            {
              key: "value",
              title: "value",
              dataIndex: "value",
              headerCellStyle: {
                display: "none",
              },
            },
          ]}
          data={Object.keys(headers).map((key) => ({
            name: key,
            value: headers[key],
          }))}
        />
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
