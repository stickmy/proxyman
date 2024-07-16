import React, { FC } from "react";
import { Sidebar } from "@/Components/Sidebar/Sidebar";
import { Outlet } from "react-router-dom";

export const Root: FC = () => {
  return (
    <div
      className="flex flex-row box-border"
      style={{ height: `calc(100% - 53px)` }}
    >
      <Sidebar />
      <div
        className="p-2 bg-content1"
        id="route-main"
        style={{ width: `calc(100% - var(--sidebar-width))` }}
      >
        <Outlet />
      </div>
    </div>
  );
};
