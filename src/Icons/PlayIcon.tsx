import { FC } from "react";
import cls from "classnames";
import { IconProps } from "./IconProps";

export const PlayIcon: FC<IconProps> = ({
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
      <path d="M251.63776 844.26752c3.76832 0 7.53664-1.04448 10.8544-3.11296l491.52-307.2c5.98016-3.74784 9.6256-10.30144 9.6256-17.36704s-3.64544-13.6192-9.6256-17.36704l-491.52-307.2c-6.32832-3.93216-14.27456-4.17792-20.7872-0.53248-6.51264 3.60448-10.5472 10.46528-10.5472 17.92l0 614.4c0 7.43424 4.03456 14.29504 10.5472 17.92C244.81792 843.40736 248.2176 844.26752 251.63776 844.26752zM272.11776 246.33344 704.512 516.58752 272.11776 786.82112 272.11776 246.33344z"></path>
    </svg>
  );
};
