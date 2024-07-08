import React from "react";
import {
  createBrowserRouter,
  createRoutesFromElements,
  RouterProvider,
  Route,
} from "react-router-dom";
import { ResizeBox } from "@arco-design/web-react";
import { Sidebar } from "@/Components/Sidebar/Sidebar";
import { Connections } from "@/Components/Connections/Connections";
import { TopBar } from "@/Components/TopBar/TopBar";
import dayjs from "dayjs";
import "dayjs/locale/zh-cn";

import "@arco-design/web-react/dist/css/arco.css";
import "./App.css";
import { Root } from "./Routes/Root";

dayjs.locale("zh-cn");

function App() {
  return (
    <div className="app w-full h-screen">
      <TopBar />
      <RouterProvider
        router={createBrowserRouter(
          createRoutesFromElements(
            <Route path="/" element={<Root />}>
              <Route element={<Connections />} index />
            </Route>
          )
        )}
      />
      {/* <ResizeBox.Split
        className="w-full"
        style={{ height: `calc(100% - 53px)` }}
        direction="horizontal"
        max="300px"
        min="180px"
        size="240px"
        panes={[
          <Sidebar />,
          <main className="main">
            <Connections />
          </main>,
        ]}
      /> */}
    </div>
  );
}

export default App;
