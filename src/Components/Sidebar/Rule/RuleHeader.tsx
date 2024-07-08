import { FC, useState } from "react";
import { IconCodeBlock, IconPlus } from "@arco-design/web-react/icon";
import { Input, Modal, Message } from "@arco-design/web-react";
import { addProcessPack } from "@/Commands/Commands";
import { usePackStore } from "./Hooks/usePacks";

export const RuleHeader: FC = () => {
  const { addPack } = usePackStore()

  const [modalVisible, setModalVisible] = useState<boolean>(false);
  const [packName, setPackName] = useState<string>();

  const onModalOk = async () => {
    if (!packName) {
      Message.info("请输入名称");
      return;
    }

    try {
      const ret = await addProcessPack(packName);
      Message.success("创建成功");
      setModalVisible(false);
      addPack(packName);
    } catch (error) {
      Message.error("创建规则失败");
    }
  };

  return (
    <>
      <div className="flex flex-row justify-between items-center">
        <span>
          <IconCodeBlock className="mr-1" />
          规则
        </span>
        <IconPlus onClick={() => setModalVisible(true)} />
      </div>
      <Modal
        visible={modalVisible}
        title="创建规则"
        onCancel={() => setModalVisible(false)}
        onOk={onModalOk}
      >
        <Input
          placeholder="输入规则名称"
          value={packName}
          onChange={(value) => setPackName(value)}
        />
      </Modal>
    </>
  );
};
