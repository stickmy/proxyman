import { usePackStore } from "@/Routes/Rule/usePacks";
import {
  Button,
  Input,
  Modal,
  ModalBody,
  ModalContent,
  ModalFooter,
  ModalHeader,
} from "@nextui-org/react";
import { toast } from "react-hot-toast";
import { FC, useState } from "react";
import { useCreatePack } from "./useCreatePack";

export const CreatePack: FC<{
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
}> = ({ isOpen, onOpenChange }) => {
  const [isInvalid, setIsInvalid] = useState<boolean>(false);
  const [invalidMessage, setInvalidMessage] = useState<string>();

  const [packName, setPackName] = useState<string>("");
  const { packs } = usePackStore();
  const { createPack, loading } = useCreatePack();

  const onPackNameChange = (name: string) => {
    setPackName(name);

    if (!name) {
      setIsInvalid(true);
      setInvalidMessage("规则组名称不能为空");
      return;
    }

    const exists = packs.some((pack) => pack.packName === name);
    if (exists) {
      setIsInvalid(true);
      setInvalidMessage(`规则组(${name})已存在`);
      return;
    }

    setIsInvalid(false);
  };

  const createPackAndClose = async (onClose: () => void) => {
    if (!packName || isInvalid) {
      toast.error("请检查规则组名称", { duration: 20000});
      return;
    }

    try {
      await createPack(packName);
      toast.success("创建成功")
      onClose();
    } catch (error) {
      toast.error(error as string);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      placement="top-center"
      backdrop="opaque"
    >
      <ModalContent>
        {(onClose) => (
          <>
            <ModalHeader className="flex flex-col gap-1 text-tiny">
              创建规则组
            </ModalHeader>
            <ModalBody>
              <Input
                size="sm"
                autoFocus
                label="规则组名称"
                placeholder="输入规则组名称"
                variant="bordered"
                value={packName}
                onValueChange={onPackNameChange}
                isInvalid={isInvalid}
                errorMessage={invalidMessage}
              />
            </ModalBody>
            <ModalFooter className="text-tiny">
              <Button color="danger" variant="flat" onPress={onClose} size="sm">
                取消
              </Button>
              <Button
                color="primary"
                onPress={() => createPackAndClose(onClose)}
                size="sm"
                isLoading={loading}
              >
                创建
              </Button>
            </ModalFooter>
          </>
        )}
      </ModalContent>
    </Modal>
  );
};
