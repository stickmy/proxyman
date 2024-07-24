import { saveValue } from "@/Commands/Commands";
import { useState } from "react";
import { useValueStore } from "./useValueStore";

export const useCreateValue = () => {
  const [loading, setLoading] = useState<boolean>(false);

  const { addValue } = useValueStore();

  const createValue = async (name: string, value = "") => {
    if (loading) return;
    setLoading(true);

    try {
      await saveValue(name, value);
      addValue(name);
    } finally {
      setLoading(false);
    }
  };

  return {
    loading,
    createValue,
  };
};
