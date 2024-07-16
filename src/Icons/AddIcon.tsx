import { FC } from "react";
import cls from "classnames";
import { IconProps } from "./IconProps";

export const AddIcon: FC<IconProps> = ({
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
      <path d="M853.333333 483.555556H540.444444V170.666667c0-15.928889-12.515556-28.444444-28.444444-28.444445s-28.444444 12.515556-28.444444 28.444445v312.888889H170.666667c-15.928889 0-28.444444 12.515556-28.444445 28.444444s12.515556 28.444444 28.444445 28.444444h312.888889V853.333333c0 15.928889 12.515556 28.444444 28.444444 28.444445s28.444444-12.515556 28.444444-28.444445V540.444444H853.333333c15.928889 0 28.444444-12.515556 28.444445-28.444444s-12.515556-28.444444-28.444445-28.444444z"></path>
    </svg>
  );
};
