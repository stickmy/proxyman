import {
  type ConnectionEvent,
  type RequestConnection,
  type ResponseConnection,
  isRequestEvent,
  isResponseEvent,
} from "@/Events/ConnectionEvents";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";

export interface Connection {
  id: string;
  request: RequestConnection;
  response?: ResponseConnection;
}

export const useConnectionStore = create<{
  connections: Connection[];
  clearConnections: () => void;
}>((set) => ({
  connections: [] as Connection[],
  clearConnections: () => set({ connections: [] }),
}));

let cacheConnQueue: RequestConnection[] = [];
const UPDATE_INTERVAL: number = 200;

let updateTimer: number | undefined = undefined;
const startUpdateConnStateLoop = () => {
  updateTimer = window.setTimeout(() => {
    if (cacheConnQueue.length) {
      const reqs = cacheConnQueue.map<Connection>((x) => ({
        id: x.id,
        request: x,
      }));
      useConnectionStore.setState(({ connections }) => ({
        connections: [...reqs, ...connections],
      }));

      cacheConnQueue = [];
    }

    startUpdateConnStateLoop();
  }, UPDATE_INTERVAL);
};

startUpdateConnStateLoop();

const processConnections = (event: ConnectionEvent) => {
  if (isRequestEvent(event)) {
    cacheConnQueue.push(event.NewRequest);
  } else if (isResponseEvent(event)) {
    const resConn = event.NewResponse;

    useConnectionStore.setState(({ connections }) => {
      const conn = connections.find((x) => x.id === resConn.id);

      if (!conn) {
        console.warn(
          `Orphan response(None matched request connection found: ${resConn.id})`,
        );
        return { connections };
      }

      const updatedConn: Connection = {
        id: conn.id,
        request: conn.request,
        response: resConn,
      };

      return {
        connections: connections.map((x) =>
          x.id === conn.id ? updatedConn : x,
        ),
      };
    });
  }
};

// @ts-ignore
const unlisten = listen<ConnectionEvent>("proxy_event", (event) => {
  const payload = event.payload;
  console.debug("connections", payload);
  processConnections(payload);
});

window.addEventListener("beforeunload", () => {
  unlisten.then((f) => f());
});

// @ts-ignore
window._unlisten = unlisten;
