import { addProcessPack } from "@/Commands/Commands";
import { usePackStore } from "@/Routes/Rule/usePacks";
import { useState } from "react";

export const useCreatePack = () => {
  const [loading, setLoading] = useState<boolean>(false);

  const { addPack } = usePackStore();

  const createPack = async (packName: string) => {
    if (loading) return;
    setLoading(true);

    try {
      const default_enable = true;
      await addProcessPack(packName, default_enable);
      addPack(packName, default_enable);
    } finally {
      setLoading(false);
    }
  };

  return {
    loading,
    createPack,
  };
};
