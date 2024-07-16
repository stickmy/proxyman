import { FC } from "react";
import cls from "classnames";
import { IconProps } from "./IconProps";

export const RightLayoutIcon: FC<IconProps> = ({
  size = 16,
  className,
  onClick,
  ...rest
}) => {
  return (
    <svg
      className={cls("text-foreground", className)}
      viewBox="0 0 1024 1024"
      version="1.1"
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      fill="currentColor"
      onClick={onClick}
      {...rest}
    >
      <path d="M896 128a42.666667 42.666667 0 0 1 42.666667 42.666667v682.666666a42.666667 42.666667 0 0 1-42.666667 42.666667H128a42.666667 42.666667 0 0 1-42.666667-42.666667V170.666667a42.666667 42.666667 0 0 1 42.666667-42.666667h768z m-256 85.333333H170.666667v597.333334h469.333333V213.333333z m213.333333 0h-128v597.333334h128V213.333333z"></path>
    </svg>
  );
};
