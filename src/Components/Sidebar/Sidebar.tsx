import React, { FC } from "react";
import { NavLink } from "react-router-dom";
import { Tooltip } from "@nextui-org/react";
import { HttpIcon, RuleIcon } from "@/Icons";
import { useTheme } from "@/Components/TopBar/useTheme";
import "./index.css";

export const Sidebar: FC = () => {
  const { theme } = useTheme();

  const getActiveColor = () => {
    return theme === "dark"
      ? "hsl(var(--nextui-success-200))"
      : "hsl(var(--nextui-success-500))";
  };

  return (
    <aside className="sidebar box-border border-r-1 border-default-100 flex-shrink-0 select-none h-full bg-content1">
      <ul className="h-full">
        <li>
          <Tooltip content="connections" placement="right">
            <NavLink
              to="/"
              className="w-full h-full flex items-center justify-center"
              style={({ isActive }) => ({
                background: isActive ? getActiveColor() : "",
              })}
            >
              <HttpIcon size={18} />
            </NavLink>
          </Tooltip>
        </li>
        <li>
          <Tooltip content="rules" placement="right">
            <NavLink
              to="/pack"
              className="w-full h-full flex items-center justify-center"
              style={({ isActive }) => ({
                background: isActive ? getActiveColor() : "",
              })}
            >
              <RuleIcon size={18} />
            </NavLink>
          </Tooltip>
        </li>
      </ul>
    </aside>
  );
};
