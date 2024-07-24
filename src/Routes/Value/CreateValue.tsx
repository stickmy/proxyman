import {
  Button,
  Input,
  Modal,
  ModalBody,
  ModalContent,
  ModalFooter,
  ModalHeader,
} from "@nextui-org/react";
import { type FC, useState } from "react";
import { toast } from "react-hot-toast";
import { useValueStore } from "./useValueStore";
import { useCreateValue } from "./useCreateValue";

export const CreateValue: FC<{
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  content?: string;
  onSuccess?: (name: string) => void;
}> = ({ content = "", isOpen, onOpenChange, onSuccess }) => {
  const [isInvalid, setIsInvalid] = useState<boolean>(false);
  const [invalidMessage, setInvalidMessage] = useState<string>();

  const [valueName, setValueName] = useState<string>("");
  const { values } = useValueStore();
  const { createValue, loading } = useCreateValue();

  const onValueNameChange = (name: string) => {
    setValueName(name);

    if (!name) {
      setIsInvalid(true);
      setInvalidMessage("值文件名称不能为空");
      return;
    }

    const exists = values.some((value) => value === name);
    if (exists) {
      setIsInvalid(true);
      setInvalidMessage(`值文件(${name})已存在`);
      return;
    }

    setIsInvalid(false);
  };

  const createValueAndClose = async (onClose: () => void) => {
    if (loading) return;

    if (!valueName || isInvalid) {
      toast.error("请检查值文件名称", { duration: 20000 });
      return;
    }

    try {
      await createValue(valueName, content);
      toast.success("创建成功");
      onClose();
      onSuccess?.(valueName);
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
              创建值文件
            </ModalHeader>
            <ModalBody>
              <Input
                size="sm"
                autoFocus
                label="值文件名称"
                placeholder="输入值文件名称"
                variant="bordered"
                value={valueName}
                onValueChange={onValueNameChange}
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
                onPress={() => createValueAndClose(onClose)}
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
