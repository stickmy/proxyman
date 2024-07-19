import { TopBar } from "@/Components/TopBar/TopBar";
import { Connections } from "@/Routes/Connections/Connections";
import dayjs from "dayjs";
import React from "react";
import { Toaster } from "react-hot-toast";
import {
  Route,
  RouterProvider,
  createBrowserRouter,
  createRoutesFromElements,
} from "react-router-dom";
import "dayjs/locale/zh-cn";
import { Pack } from "./Routes/Pack/Pack";
import { Root } from "./Routes/Root";
import { Rule } from "./Routes/Rule/Rule";
import "./App.css";

dayjs.locale("zh-cn");

function App() {
  return (
    <div className="w-full h-screen">
      <Toaster
        toastOptions={{
          className: "text-tiny",
        }}
      />
      <TopBar />
      <RouterProvider
        router={createBrowserRouter(
          createRoutesFromElements(
            <Route path="/" element={<Root />}>
              <Route element={<Connections />} index />
              <Route element={<Pack />} path="pack/:packName?">
                <Route element={<Rule />} path=":mode?" />
              </Route>
            </Route>,
          ),
        )}
      />
    </div>
  );
}

export default App;
