import React, { FC } from "react";
import * as Separator from "@radix-ui/react-separator";

export const LabelSeparator: FC<{
  label: string;
}> = ({ label }) => {
  return (
    <div className="flex flex-row items-center my-[4px]">
      <Separator.Root className="bg-gray6 inline-block data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full " />
      <span className="inline-block mx-[8px]">{label}</span>
      <Separator.Root className="bg-gray6 inline-block data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full " />
    </div>
  );
};
