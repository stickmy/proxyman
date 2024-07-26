import {
  removeProcessorPack,
  updateProcessPackStatus,
} from "@/Commands/Commands";
import cls from "classnames";
import { AddIcon, DeleteIcon } from "@/Icons";
import { Button, Switch, useDisclosure } from "@nextui-org/react";
import { type ChangeEvent, type FC, type MouseEvent, useEffect } from "react";
import toast from "react-hot-toast";
import { NavLink, Outlet, useNavigate, useParams } from "react-router-dom";
import { usePackStore } from "./usePacks";
import { CreatePack } from "./CreatePack";

export const Pack: FC = () => {
  const navigate = useNavigate();
  const { packName } = useParams<{ packName: string }>();

  const packStore = usePackStore();
  const { packs } = packStore;

  useEffect(() => {
    // 如果路由中没有 packName, 则跳转第一个 pack 为默认路由
    if (!packName && packs.length) {
      navigate(`/pack/${packs[0].packName}`);
    } else if (packs.length && packs.every((x) => x.packName !== packName)) {
      // packs 中没有 packName, 可能是该 pack 被删除了
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

  const removePack = async (evt: MouseEvent<HTMLButtonElement>) => {
    evt.nativeEvent.stopImmediatePropagation();
    evt.stopPropagation();
    evt.preventDefault();

    const packName = evt.currentTarget.dataset.packname;
    if (!packName) return;

    try {
      await removeProcessorPack(packName);
      packStore.removePack(packName);
    } catch (error) {
      toast.error(`删除失败: ${error}`);
    }
  };

  const onCreatedSuccess = (name: string) => {
    navigate(`/pack/${name}`);
  };

  const { isOpen, onOpenChange, onOpen } = useDisclosure();

  return (
    <div className="flex flex-row w-full h-full">
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
          创建规则组
        </Button>
        <CreatePack
          isOpen={isOpen}
          onOpenChange={onOpenChange}
          onSuccess={onCreatedSuccess}
        />
        <ul>
          {packs.map((pack) => (
            <li key={pack.packName} className="[&:not(:last-child)]:mb-2">
              <NavLink
                to={`/pack/${pack.packName}`}
                className="py-2 px-2 text-sm rounded flex flex-row items-center justify-between hover:bg-default-100"
                style={({ isActive }) => ({
                  background: isActive ? "hsl(var(--nextui-default-100))" : "",
                })}
              >
                <span>{pack.packName}</span>
                <div className="flex-row flex items-center">
                  <Switch
                    className="mr-2"
                    classNames={{
                      base: cls(
                        "inline-flex flex-row-reverse max-w-md hover:bg-content2 items-center",
                        "justify-between cursor-pointer rounded-lg gap-2 border-transparent",
                      ),
                      wrapper: "p-0 h-3 overflow-visible mr-0 w-8",
                      thumb: cls(
                        "w-4 h-4 shadow-lg",
                        "group-data-[hover=true]:border-primary",
                        //selected
                        "group-data-[selected=true]:ml-4",
                        // pressed
                        "group-data-[pressed=true]:w-4",
                        "group-data-[selected]:group-data-[pressed]:ml-2",
                      ),
                    }}
                    color="success"
                    isSelected={pack.enable}
                    onChange={(evt) => onSwitch(evt, pack.packName)}
                  />
                  <Button
                    isIconOnly
                    disableRipple
                    disableAnimation
                    className="bg-transparent !text-tiny !outline-0 hover:text-primary-400 h-4 !w-fit min-w-2"
                    data-packname={pack.packName}
                    onClick={removePack}
                  >
                    <DeleteIcon size={16} className="text-default-600" />
                  </Button>
                </div>
              </NavLink>
            </li>
          ))}
        </ul>
      </div>

      <Outlet />
    </div>
  );
};
