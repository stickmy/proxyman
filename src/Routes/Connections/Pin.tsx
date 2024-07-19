import { PinIcon } from "@/Icons";
import { usePinUriStore } from "@/Store/PinUriStore";
import { Button } from "@nextui-org/react";
import cls from "classnames";
import type { FC, MouseEvent } from "react";

export const Pin: FC<{
  uri: string;
}> = ({ uri }) => {
  const { currentPin, pinUri, unpinUri, pins } = usePinUriStore();

  const onPinClick = (evt: MouseEvent<HTMLButtonElement>) => {
    evt.nativeEvent.stopImmediatePropagation();
    evt.stopPropagation();

    const uri = evt.currentTarget.dataset.uri!;

    pins.includes(uri) ? unpinUri(uri) : pinUri(uri);
  };

  // svg 事件会导致阻止不了冒泡, 从而触发 Table 的 onSelectionChange. 而使用 nextui 的 Button 可以
  return (
    <Button
      isIconOnly
      disableRipple
      disableAnimation
      className="bg-transparent !outline-none"
      onClick={onPinClick}
      data-uri={uri}
    >
      <PinIcon
        className={cls(
          "transition-transform",
          pins.includes(uri) ? "-rotate-45 pin" : "",
        )}
      />
    </Button>
  );
};
