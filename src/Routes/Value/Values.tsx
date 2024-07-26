import { Button, useDisclosure } from "@nextui-org/react";
import { useEffect, type FC, type MouseEvent } from "react";
import { AddIcon, DeleteIcon } from "@/Icons";
import { NavLink, Outlet, useNavigate, useParams } from "react-router-dom";
import { CreateValue } from "./CreateValue";
import { removeValue } from "@/Commands/Commands";
import toast from "react-hot-toast";
import { useValueStore } from "./useValueStore";

export const Values: FC = () => {
  const { name } = useParams<{ name: string }>();
  const navigate = useNavigate();

  const valueStore = useValueStore();
  const { values } = valueStore;

  useEffect(() => {
    // 重定向到第一个 item
    if (!name && values.length) {
      navigate(`/value/${values[0]}`);
    } else if (values.length && values.every((x) => x !== name)) {
      // name 不在 values 中, 可能是 name 被删除了
      navigate(`/value/${values[0]}`);
    }
  }, [name, navigate, values]);

  const onRemoveValue = async (evt: MouseEvent<HTMLButtonElement>) => {
    evt.nativeEvent.stopImmediatePropagation();
    evt.stopPropagation();
    evt.preventDefault();

    const valueName = evt.currentTarget.dataset.valuename;
    if (!valueName) return;

    try {
      await removeValue(valueName);
      valueStore.removeValue(valueName);
    } catch (error) {
      toast.error(`删除失败: ${error}`);
    }
  };

  const onCreatedSuccess = (name: string) => {
    navigate(name);
  };

  const { isOpen, onOpenChange, onOpen } = useDisclosure();

  return (
    <div className="w-full h-full flex flex-row">
      <div className="flex flex-col h-full border-r-1 border-default-100 pr-2 w-[240px] flex-shrink-0">
        <Button
          fullWidth
          variant="faded"
          radius="full"
          endContent={<AddIcon />}
          className="mb-4 hover:border-default-300"
          size="sm"
          onPress={onOpen}
        >
          创建值文件
        </Button>
        <CreateValue
          onOpenChange={onOpenChange}
          isOpen={isOpen}
          onSuccess={onCreatedSuccess}
        />
        <ul>
          {values.map((value) => (
            <li key={value}>
              <NavLink
                to={value}
                className="py-2 px-2 text-sm rounded flex flex-row items-center justify-between hover:bg-default-100"
                style={({ isActive }) => ({
                  background: isActive ? "hsl(var(--nextui-default-100))" : "",
                })}
              >
                <span>{value}</span>
                <Button
                  isIconOnly
                  disableRipple
                  disableAnimation
                  className="bg-transparent !text-tiny !outline-0 hover:text-primary-400 h-4 !w-fit min-w-2"
                  data-valuename={value}
                  onClick={onRemoveValue}
                >
                  <DeleteIcon size={16} className="text-default-600" />
                </Button>
              </NavLink>
            </li>
          ))}
        </ul>
      </div>
      <Outlet />
    </div>
  );
};
