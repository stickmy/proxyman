export function isJson(body: string) {
  try {
    JSON.parse(body);
    return true;
  } catch {
    return false;
  }
}

export function isJsonp(body: string): string | false {
  const re = /^[^(]*\((.*)\)$/;
  const parts = body.match(re);

  if (parts && parts[1]) {
    const json = isJson(parts[1]);
    return json ? parts[1] : false;
  }

  return false;
}

export const tryStringifyWithSpaces = (str: string): string => {
  try {
    return JSON.stringify(JSON.parse(str), null, 2);
  } catch {
    return str;
  }
};
