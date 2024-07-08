import React, { FC } from "react";
import { Sidebar } from "@/Components/Sidebar/Sidebar";
import { Outlet } from "react-router-dom";

export const Root: FC = () => {
  return (
    <div
      className="flex flex-row p-2 box-border"
      style={{ height: `calc(100% - 53px)` }}
    >
      <div className="w-[300px]">
        <Sidebar />
      </div>
      <div className="w-full ml-2">
        <Outlet />
      </div>
    </div>
  );
};
