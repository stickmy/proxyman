import React from "react";
import { ResizeBox } from "@arco-design/web-react";
import { Sidebar } from "@/Components/Sidebar/Sidebar";
import { Connections } from "@/Components/Connections/Connections";
import dayjs from "dayjs";
import "dayjs/locale/zh-cn";

import "@arco-design/web-react/dist/css/arco.css";
import "./App.css";

dayjs.locale("zh-cn");

function App() {
  return (
    <div className="app w-full h-screen flex flex-row">
      <ResizeBox.Split
        className="w-full h-full"
        direction="horizontal"
        max="600px"
        min="240px"
        size="260px"
        panes={[
          <Sidebar />,
          <main className="main bg-gray2 w-full relative pl-1">
            <Connections />
          </main>,
        ]}
      />
    </div>
  );
}

export default App;
