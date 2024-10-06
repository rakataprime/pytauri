import { invoke } from "@tauri-apps/api/core";

const PY_INVOKE_TAURI_CMD = "plugin:pytauri|pyfunc";
const PY_INVOKE_HEADER = "pyfunc";

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

// we dont support `number[]`, because it will make type checking hard
type RawHandlerArgType = ArrayBuffer | Uint8Array;
type RawHandlerReturnType = ArrayBuffer;

async function rawPyInvoke(
    funcName: string,
    arg: RawHandlerArgType
): Promise<RawHandlerReturnType> {
    const invokePromise = invoke(PY_INVOKE_TAURI_CMD, arg, {
        headers: { [PY_INVOKE_HEADER]: funcName },
    });

    // development-time type checking to make sure pytauri ipc implementation is correct
    if (process.env.NODE_ENV === "development") {
        return invokePromise.then((response) => {
            if (!(response instanceof ArrayBuffer)) {
                throw new Error(
                    "response is not ArrayBuffer. This is not your fault, \
                    it's a bug for pytauri, please report this issue."
                );
            }
            return response;
        });
    } else {
        return invokePromise as Promise<RawHandlerReturnType>;
    }
}

export async function pyInvoke(
    funcName: string,
    arg: object | RawHandlerArgType
): ReturnType<typeof rawPyInvoke> {
    let argEncoded: RawHandlerArgType;

    if (!(arg instanceof ArrayBuffer) && !(arg instanceof Uint8Array)) {
        const argJson = JSON.stringify(arg);
        argEncoded = textEncoder.encode(argJson);
    } else {
        argEncoded = arg;
    }
    return rawPyInvoke(funcName, argEncoded);
}

export function fromJson(
    json: ArrayBuffer | Uint8Array
): ReturnType<typeof JSON.parse> {
    const jsonStr = textDecoder.decode(json);
    return JSON.parse(jsonStr);
}
