import { updateProcessPackStatus } from "@/Commands/Commands";
import { AddIcon } from "@/Icons";
import { usePackStore } from "@/Routes/Rule/usePacks";
import { Button, Switch, useDisclosure } from "@nextui-org/react";
import { type ChangeEvent, type FC, useEffect } from "react";
import toast from "react-hot-toast";
import { NavLink, Outlet, useNavigate, useParams } from "react-router-dom";
import { CreatePack } from "./CreatePack";

export const Pack: FC = () => {
  const navigate = useNavigate();
  const { packName } = useParams<{ packName: string }>();

  const packStore = usePackStore();
  const { packs } = packStore;

  useEffect(() => {
    // 如果路由中没有 packName, 则跳转第一个 pack 为默认路由
    if (!packName && packs.length !== 0) {
      navigate(`/pack/${packs[0].packName}`);
    }
  }, [packName, packs, navigate]);

  const onSwitch = async (
    evt: ChangeEvent<HTMLInputElement>,
    packName: string,
  ) => {
    evt.stopPropagation();

    const checked = evt.target.checked;

    try {
      const ret = await updateProcessPackStatus(packName, checked);
      packStore.updatePackStatus(packName, checked);
    } catch (error) {
      toast.error(`${checked ? "开启" : "关闭"}规则失败`);
    }
  };

  const { isOpen, onOpenChange, onOpen } = useDisclosure();

  return (
    <div className="flex flex-row w-full h-full">
      <div className="flex flex-col h-full border-r-1 border-default-100 pr-2 w-[200px] flex-shrink-0">
        <Button
          fullWidth
          variant="faded"
          radius="full"
          endContent={<AddIcon />}
          className="mb-4 hover:border-default-300"
          size="sm"
          onPress={onOpen}
        >
          创建规则组
        </Button>
        <CreatePack isOpen={isOpen} onOpenChange={onOpenChange} />
        <ul>
          {packs.map(pack => (
            <li key={pack.packName} className="[&:not(:last-child)]:mb-2">
              <NavLink
                to={`/pack/${pack.packName}`}
                className="py-2 pl-2 text-sm rounded flex flex-row items-center justify-between hover:bg-default-100"
                style={({ isActive }) => ({
                  background: isActive ? "hsl(var(--nextui-default-100))" : "",
                })}
              >
                <span>{pack.packName}</span>
                <Switch
                  color="success"
                  size="sm"
                  isSelected={pack.enable}
                  onChange={evt => onSwitch(evt, pack.packName)}
                />
              </NavLink>
            </li>
          ))}
        </ul>
      </div>

      <Outlet />
    </div>
  );
};
