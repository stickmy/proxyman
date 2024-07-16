import { FC } from "react";
import cls from "classnames";
import { IconProps } from "./IconProps";

export const HttpIcon: FC<IconProps> = ({
  size = 16,
  className,
  onClick,
  ...rest
}) => {
  return (
    <svg
      className={cls(`text-foreground`, className)}
      viewBox="0 0 1110 1024"
      version="1.1"
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      fill="currentColor"
      onClick={onClick}
      {...rest}
    >
      <path d="M946.939661 418.312678a23.170169 23.170169 0 0 0 23.100746-23.239593V116.197966c0-64.04339-51.781424-116.197966-115.486373-116.197966H115.477695C51.781424 0 0 52.154576 0 116.197966v790.154848c0 64.04339 51.781424 116.197966 115.477695 116.197966h739.076339c63.696271 0 115.477695-52.154576 115.477695-116.197966V766.915254a23.170169 23.170169 0 0 0-23.092068-23.239593 23.170169 23.170169 0 0 0-23.100746 23.239593v139.43756c0 38.44339-31.084475 69.71878-69.284881 69.718779H115.477695c-38.200407 0-69.284881-31.284068-69.284881-69.718779V232.395932h877.654779v162.677153a23.170169 23.170169 0 0 0 23.092068 23.239593zM46.192814 185.916746V116.197966c0-38.44339 31.084475-69.71878 69.284881-69.71878h739.076339c38.200407 0 69.293559 31.284068 69.293559 69.71878v69.71878H46.192814z m69.284881-46.479187a23.170169 23.170169 0 0 0 23.100746-23.239593 23.170169 23.170169 0 0 0-23.100746-23.239593 23.170169 23.170169 0 0 0-23.092068 23.239593 23.170169 23.170169 0 0 0 23.092068 23.239593z m92.385627 0a23.170169 23.170169 0 0 0 23.100746-23.239593 23.170169 23.170169 0 0 0-23.100746-23.239593 23.170169 23.170169 0 0 0-23.092068 23.239593 23.170169 23.170169 0 0 0 23.092068 23.239593z m92.385627 0a23.170169 23.170169 0 0 0 23.100746-23.239593 23.170169 23.170169 0 0 0-23.100746-23.239593 23.170169 23.170169 0 0 0-23.092068 23.239593 23.170169 23.170169 0 0 0 23.092068 23.239593zM254.056136 464.791864a23.170169 23.170169 0 0 1 23.100745 23.248272v185.916745a23.170169 23.170169 0 0 1-23.100745 23.239594 23.170169 23.170169 0 0 1-23.092068-23.239594v-69.718779H138.578441v69.718779a23.170169 23.170169 0 0 1-23.100746 23.239594 23.170169 23.170169 0 0 1-23.092068-23.239594V488.040136a23.170169 23.170169 0 0 1 23.092068-23.248272 23.170169 23.170169 0 0 1 23.100746 23.248272v69.718779h92.385627v-69.718779a23.170169 23.170169 0 0 1 23.092068-23.248272z m184.771254 0a23.170169 23.170169 0 0 1 23.100746 23.239594 23.170169 23.170169 0 0 1-23.100746 23.239593h-23.100746v162.68583a23.170169 23.170169 0 0 1-23.08339 23.239594 23.170169 23.170169 0 0 1-23.100746-23.239594V511.279729h-23.100745a23.170169 23.170169 0 0 1-23.100746-23.239593 23.170169 23.170169 0 0 1 23.100746-23.248272h92.385627z m184.771254 0a23.170169 23.170169 0 0 1 23.092068 23.248272 23.170169 23.170169 0 0 1-23.092068 23.239593h-23.100746v162.677152a23.170169 23.170169 0 0 1-23.092067 23.239594 23.170169 23.170169 0 0 1-23.100746-23.239594V511.279729h-23.092068a23.170169 23.170169 0 0 1-23.100746-23.239593 23.170169 23.170169 0 0 1 23.100746-23.248272H623.598644z m138.569763 0c38.209085 0 69.293559 31.284068 69.293559 69.71878 0 38.44339-31.084475 69.727458-69.293559 69.727458h-23.08339v69.718779a23.170169 23.170169 0 0 1-23.100746 23.239594 23.170169 23.170169 0 0 1-23.100746-23.239594V488.040136a23.170169 23.170169 0 0 1 23.100746-23.248272h46.184136z m0 92.958373c12.704542 0 23.100746-10.413559 23.100746-23.239593a23.204881 23.204881 0 0 0-23.100746-23.239593h-23.092068v46.479186h23.092068z m328.938305-92.255457a23.196203 23.196203 0 0 1 16.817898 28.168678l-46.192813 185.916745a23.048678 23.048678 0 0 1-27.995119 16.922034 23.196203 23.196203 0 0 1-16.80922-28.177356l46.184135-185.916745c3.054644-12.496271 15.429424-20.167593 27.995119-16.922034z m-92.385627 0a23.196203 23.196203 0 0 1 16.817898 28.168678l-46.192814 185.916745a23.048678 23.048678 0 0 1-27.995118 16.922034 23.196203 23.196203 0 0 1-16.80922-28.177356l46.192813-185.916745c3.037288-12.496271 15.377356-20.167593 27.986441-16.922034z m-97.974238 92.264135a23.170169 23.170169 0 0 1-23.092067-23.248271 23.170169 23.170169 0 0 1 23.092067-23.239593 23.170169 23.170169 0 0 1 23.100746 23.239593 23.170169 23.170169 0 0 1-23.100746 23.248271z m0 92.958373a23.170169 23.170169 0 0 1-23.092067-23.239593 23.170169 23.170169 0 0 1 23.092067-23.239593 23.170169 23.170169 0 0 1 23.100746 23.239593 23.170169 23.170169 0 0 1-23.100746 23.239593z"></path>
    </svg>
  );
};