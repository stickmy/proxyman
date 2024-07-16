import { FC } from "react";
import cls from "classnames";
import { IconProps } from "./IconProps";

export const PauseIcon: FC<IconProps> = ({
  size = 16,
  className,
  onClick,
  ...rest
}) => {
  return (
    <svg
      className={cls(`text-foreground`, className)}
      viewBox="0 0 1024 1024"
      version="1.1"
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      fill="currentColor"
      onClick={onClick}
      {...rest}
    >
      <path d="M384 736c-19.2 0-32-12.8-32-32V320c0-19.2 12.8-32 32-32s32 12.8 32 32v384c0 19.2-12.8 32-32 32zM640 736c-19.2 0-32-12.8-32-32V320c0-19.2 12.8-32 32-32s32 12.8 32 32v384c0 19.2-12.8 32-32 32z"></path>
    </svg>
  );
};
