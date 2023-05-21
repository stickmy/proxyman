import { create } from "zustand";
import { unstable_batchedUpdates } from "react-dom";
import { RuleMode } from "@/Events/ConnectionEvents";

export interface RuleContextType {
  editRule: (mode: RuleMode, prefilling?: string) => void;
}

export const useRuleActions = create<RuleContextType>((set) => ({
  editRule: () => {},
}));

export const setRuleEditHandler = (handler: RuleContextType["editRule"]) =>
  unstable_batchedUpdates(() => {
    useRuleActions.setState((state) => ({ editRule: handler }));
  });
