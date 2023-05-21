import React from "react";

import { usePinUriStore } from "@/Store/PinUriStore";
import { IconClose } from "@arco-design/web-react/icon";

export const Pin = () => {
  const { pins, unpinUri, setCurrentPin, currentPin } = usePinUriStore();

  return (
    <ul>
      {pins.map((uri, index) => (
        <li
          key={index}
          className="hover:text-blue9 transition flex flex-row items-center cursor-pointer data-[pined=true]:text-blue11"
          data-pined={currentPin === uri}
          onClick={() => setCurrentPin(uri === currentPin ? undefined : uri)}
        >
          {/*<IconClose className="shrink-0 mr-1" />*/}
          <span className="break-all">{uri}</span>
        </li>
      ))}
    </ul>
  );
};
