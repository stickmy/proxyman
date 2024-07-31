import cls from "classnames";
import type { FC } from "react";
import type { IconProps } from "./IconProps";

export const SecureIcon: FC<IconProps> = ({
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
      p-id="14332"
      width={size}
      height={size}
      fill="currentColor"
      onClick={onClick}
      {...rest}
    >
      <path d="M498.19648 61.44a30.72 30.72 0 0 1 17.2032 0l368.47616 107.64288a30.72 30.72 0 0 1 22.1184 29.4912v220.73344a569.67168 569.67168 0 0 1-389.57056 540.42624 30.72 30.72 0 0 1-19.456 0A569.7536 569.7536 0 0 1 107.3152 419.2256V198.57408a30.72 30.72 0 0 1 22.1184-29.4912L498.19648 61.44zM168.7552 221.5936v197.59104a508.3136 508.3136 0 0 0 337.96096 478.94528 508.23168 508.23168 0 0 0 337.92-478.8224V221.5936L506.75712 122.88 168.7552 221.5936zM733.184 355.9424a30.72 30.72 0 0 1 0 43.4176l-245.76 245.76a30.72 30.72 0 0 1-43.4176 0l-143.36-143.36a30.72 30.72 0 0 1 43.4176-43.4176l121.6512 121.6512 224.0512-224.0512a30.72 30.72 0 0 1 43.4176 0z" />
    </svg>
  );
};
