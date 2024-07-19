import {
  type ConnectionEvent,
  type RequestConnection,
  type ResponseConnection,
  isRequestEvent,
  isResponseEvent,
} from "@/Events/ConnectionEvents";
import { listen } from "@tauri-apps/api/event";
import { unstable_batchedUpdates } from "react-dom";
import { create } from "zustand";

export interface Connection {
  id: string;
  request: RequestConnection;
  response?: ResponseConnection;
}

export const useConnectionStore = create<{
  connections: Connection[];
  clearConnections: () => void;
}>(set => ({
  connections: [] as Connection[],
  clearConnections: () => set({ connections: [] }),
}));

const processConnections = (event: ConnectionEvent) => {
  if (isRequestEvent(event)) {
    const reqConn = event.NewRequest;

    // TODO: perf optimization
    unstable_batchedUpdates(() =>
      useConnectionStore.setState(({ connections }) => {
        if (connections.find(x => x.id === reqConn.id)) {
          console.warn("Duplicate connection:", reqConn.uri);
        }

        const conn: Connection = {
          id: reqConn.id,
          request: reqConn,
        };

        return { connections: [...connections, conn] };
      }),
    );
  } else if (isResponseEvent(event)) {
    const resConn = event.NewResponse;

    unstable_batchedUpdates(() => {
      useConnectionStore.setState(({ connections }) => {
        const conn = connections.find(x => x.id === resConn.id);

        if (!conn) {
          console.warn(
            "Orphan response(None matched request connection found)",
          );
          return { connections: connections };
        }

        const updatedConn: Connection = {
          id: conn.id,
          request: conn.request,
          response: resConn,
        };

        return {
          connections: connections.map(x =>
            x.id === conn.id ? updatedConn : x,
          ),
        };
      });
    });
  }
};

// @ts-ignore
const unlisten = listen<ConnectionEvent>("proxy_event", event => {
  const payload = event.payload;
  console.debug("connections", payload);
  processConnections(payload);
});

window.addEventListener("beforeunload", () => {
  unlisten.then(f => f());
});

// @ts-ignore
window._unlisten = unlisten;
