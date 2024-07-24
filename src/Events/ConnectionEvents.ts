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
  /**
   * 实际请求的 uri, 比如 A redirect to B, request.uri 是 A, response.uri 是 B
   */
  uri: string;
  status: number;
  version: string;
  headers: Record<string, string>;
  body: string;
  effects?: Record<
    string,
    Array<{
      name: string;
      info: Record<string, string>;
    }>
  >;
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
