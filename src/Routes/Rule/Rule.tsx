import { FC, useEffect } from "react";
import { useRuleEditor } from "@/Routes/Rule/useRuleEditor";
import { RuleMode } from "@/Events/ConnectionEvents";
import { NavLink, useNavigate, useParams } from "react-router-dom";

const RULE_EDITOR_ELEM_ID = "rule-editor";

export const Rule: FC = () => {
  const navigate = useNavigate();
  const { mode, packName } = useParams<{ mode?: RuleMode; packName: string }>();

  const { setMode, setPackName } = useRuleEditor(
    RULE_EDITOR_ELEM_ID,
    packName!
  );

  useEffect(() => {
    if (packName) {
      setPackName(packName);
    }
    if (mode) {
      setMode(mode);
    }
  }, [mode, packName]);

  // 首页路由设置
  useEffect(() => {
    if (!mode) {
      navigate(RuleMode.Redirect, { replace: true });
    }
  }, [mode, packName]);

  return (
    <div className="w-full h-full flex flex-row">
      <ul className="w-[180px] px-2 flex-shrink-0">
        {ruleModes.map((mode) => (
          <li key={mode.mode} className="[&:not(:last-child)]:mb-2">
            <NavLink
              to={`/pack/${packName}/${mode.mode}`}
              className="block px-2 py-2 rounded hover:bg-default-100 text-tiny"
              style={({ isActive }) => ({
                background: isActive ? "hsl(var(--nextui-default-100))" : "",
              })}
            >
              {mode.label}
            </NavLink>
          </li>
        ))}
      </ul>
      <div
        id={RULE_EDITOR_ELEM_ID}
        className="w-full h-full"
        style={{ display: mode ? "block" : "none" }}
      ></div>
    </div>
  );
};

const ruleModes = [
  {
    label: "重定向",
    mode: RuleMode.Redirect,
  },
  {
    label: "延时",
    mode: RuleMode.Delay,
  },
];
