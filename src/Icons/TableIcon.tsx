import { FC } from "react";
import cls from "classnames";
import { IconProps } from "./IconProps";

export const TableIcon: FC<IconProps> = ({
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
      <path d="M896 128a85.333333 85.333333 0 0 1 85.333333 85.333333v597.333334a85.333333 85.333333 0 0 1-85.333333 85.333333h-213.333333v2.090667h-42.666667V896H384v2.090667H341.333333V896H128a85.333333 85.333333 0 0 1-85.333333-85.333333V213.333333a85.333333 85.333333 0 0 1 85.333333-85.333333h768zM341.333333 725.333333H85.333333v85.333334a42.666667 42.666667 0 0 0 37.674667 42.368L128 853.333333h213.333333v-128z m298.666667 0H384v128h256v-128z m298.666667 0h-256v128h213.333333a42.666667 42.666667 0 0 0 42.368-37.674666L938.666667 810.666667v-85.333334zM341.333333 512H85.333333v170.666667h256v-170.666667z m298.666667 0H384v170.666667h256v-170.666667z m298.666667 0h-256v170.666667h256v-170.666667zM341.333333 298.666667H85.333333v170.666666h256V298.666667z m298.666667 0H384v170.666666h256V298.666667z m298.666667 0h-256v170.666666h256V298.666667z"></path>
    </svg>
  );
};