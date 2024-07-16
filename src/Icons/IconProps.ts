import { HTMLAttributes, MouseEvent } from "react";

export interface IconProps extends HTMLAttributes<SVGElement> {
  size?: number;
  className?: string;
  onClick?: (event: MouseEvent<SVGElement>) => void;
}
