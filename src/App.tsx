import React from "react";
import {
  createBrowserRouter,
  createRoutesFromElements,
  RouterProvider,
  Route,
} from "react-router-dom";
import { Toaster } from "react-hot-toast";
import { Connections } from "@/Routes/Connections/Connections";
import { TopBar } from "@/Components/TopBar/TopBar";
import dayjs from "dayjs";
import "dayjs/locale/zh-cn";
import { Root } from "./Routes/Root";
import { Pack } from "./Routes/Pack/Pack";
import { Rule } from "./Routes/Rule/Rule";
import "./App.css";

dayjs.locale("zh-cn");

function App() {
  return (
    <div className="w-full h-screen">
      <Toaster toastOptions={{
        className: "text-tiny"
      }} />
      <TopBar />
      <RouterProvider
        router={createBrowserRouter(
          createRoutesFromElements(
            <Route path="/" element={<Root />}>
              <Route element={<Connections />} index />
              <Route element={<Pack />} path="pack/:packName?">
                <Route element={<Rule />} path=":mode?" />
              </Route>
            </Route>
          )
        )}
      />
    </div>
  );
}

export default App;
