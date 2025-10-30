import * as passer from 'passer_wasm';
import { decode as msgDecode } from '@msgpack/msgpack';

import * as util from '../Util';

export interface Decoded {
  length: number;
  data: Uint8Array[];
}

export const decode = (data: ArrayBuffer) => {
  const decoded = msgDecode(data) as Uint8Array[];
  return {
    length: decoded.map(datum => datum.length).reduce((a, b) => a + b, 0),
    data: decoded,
  };
};

export const decrypt = async (key: string, decoded: Decoded) => {
  return decryptWithKey(passer.Key.from_base64(key), decoded);
};

export const decryptWithKey = async (key: passer.Key, decoded: Decoded) => {
  return util.yieldProcessing().then(() => decoded.data.map(datum => key.decrypt(datum)));
};
