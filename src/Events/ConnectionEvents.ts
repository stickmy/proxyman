export enum RuleMode {
  Redirect = "Redirect",
  Delay = "Delay",
  Response = "Response",
}

interface BaseConnection {
  id: string;
  time: number;
}

export interface RequestConnection extends BaseConnection {
  method: string;
  uri: string;
  body: string;
  headers: Record<string, string>;
  version: string;
}
export interface ResponseConnection extends BaseConnection {
  body: string;
  headers: Record<string, string>;
  status: number;
  version: string;
  hitRules?: Record<string, RuleMode[]>;
}

export interface RequestEvent {
  NewRequest: RequestConnection;
}

export interface ResponseEvent {
  NewResponse: ResponseConnection;
}

/**
 * Received from rust.
 */
export type ConnectionEvent = RequestEvent | ResponseEvent;

export const isRequestEvent = (
  event: ConnectionEvent,
): event is RequestEvent => {
  return "NewRequest" in event;
};

export const isResponseEvent = (
  event: ConnectionEvent,
): event is ResponseEvent => {
  return "NewResponse" in event;
};
