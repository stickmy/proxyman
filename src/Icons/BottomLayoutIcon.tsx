import { FC } from "react";
import cls from "classnames";
import { IconProps } from "./IconProps";

export const BottomLayoutIcon: FC<IconProps> = ({
  className,
  size = 16,
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
      <path d="M884.48 57.6H139.52a46.72 46.72 0 0 0-46.72 46.72v802.56a46.72 46.72 0 0 0 46.72 46.72h744.96a46.72 46.72 0 0 0 46.72-46.72V104.32a46.72 46.72 0 0 0-46.72-46.72z m-23.68 826.24H163.2v-168.96h697.6z m0-238.72H163.2V128h697.6z"></path>
    </svg>
  );
};
