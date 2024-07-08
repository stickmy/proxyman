import { FC, PropsWithChildren, ReactNode } from "react";
import "./index.css";

export const Title: FC<
  PropsWithChildren<{
    tip?: ReactNode;
  }>
> = ({ children, tip }) => {
  return (
    <div className="sidebar-title">
      {children}
      {tip && <div className="sidebar-tip">{tip}</div>}
    </div>
  );
};
