import { useMemo, FC } from "react";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  ModalProps,
  useDisclosure,
} from "@nextui-org/react";
import { TRANSITION_EASINGS } from "@nextui-org/framer-utils";

type DrawerProps = Omit<ModalProps, "placement" | "scrollBehavior"> & {
  placement?: "top" | "right" | "bottom" | "left";
  scrollBehavior?: "inside" | "outside";
};

export const Drawer: FC<DrawerProps> = ({
  placement = "right",
  scrollBehavior = "inside",
  size = "md",
  motionProps: drawerMotionProps,
  children,
  ...props
}) => {
  const { isOpen, onOpen, onOpenChange } = useDisclosure({
    defaultOpen: false,
  });

  const motionProps = useMemo(() => {
    if (drawerMotionProps !== void 0) return drawerMotionProps;

    const key = placement === "left" || placement === "right" ? "x" : "y";

    return {
      variants: {
        enter: {
          [key]: 0,
          transition: {
            [key]: {
              bounce: 0,
              duration: 0.3,
              ease: TRANSITION_EASINGS.ease,
            },
          },
        },
        exit: {
          [key]: placement === "top" || placement === "left" ? "-100%" : "100%",
          transition: {
            [key]: {
              bounce: 0,
              duration: 0.3,
              ease: TRANSITION_EASINGS.ease,
            },
          },
        },
      },
    };
  }, [placement, drawerMotionProps]);

  const base = useMemo(() => {
    const sizeSource = {
      xs: "max-h-[20rem]",
      sm: "max-h-[24rem]",
      md: "max-h-[28rem]",
      lg: "max-h-[32rem]",
      xl: "max-h-[36rem]",
      "2xl": "max-h-[42rem]",
      "3xl": "max-h-[48rem]",
      "4xl": "max-h-[56rem]",
      "5xl": "max-h-[64rem]",
      full: "max-h-full",
    };
    switch (placement) {
      case "right": {
        return `absolute inset-y-0 right-0 m-0 sm:m-0 max-h-[none] overflow-y-auto rounded-r-none`;
      }
      case "left": {
        return `absolute inset-y-0 left-0 m-0 sm:m-0 max-h-[none] overflow-y-auto rounded-l-none`;
      }
      case "top": {
        return `absolute inset-x-0 top-0 m-0 sm:m-0 max-w-[none] ${sizeSource[size]} overflow-y-auto rounded-t-none`;
      }
      case "bottom": {
        return `absolute inset-x-0 bottom-0 m-0 sm:m-0 max-w-[none] ${sizeSource[size]} overflow-y-auto rounded-b-none`;
      }
    }
  }, [placement, size]);

  return (
    <Modal
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      {...props}
      size={size}
      scrollBehavior={scrollBehavior}
      classNames={{
        base: base,
      }}
      motionProps={motionProps}
    >
      <ModalContent>
        {(onClose) => <ModalBody>{children}</ModalBody>}
      </ModalContent>
    </Modal>
  );
};

Drawer.displayName = "Drawer";
