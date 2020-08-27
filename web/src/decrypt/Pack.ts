import * as passer from 'passer'
import { decode as msgDecode } from '@msgpack/msgpack'

import * as util from '../Util'

export interface Decoded {
  length: number
  data: Uint8Array[]
}

export const decode = (data: ArrayBuffer) => {
  const decoded = msgDecode(data) as Uint8Array[]
  return {
    length: decoded.map(datum => datum.length).reduce((a, b) => a + b, 0),
    data: decoded,
  }
}

export const decrypt = async (key: string, decoded: Decoded) => {
  return util.yieldProcessing()
  .then(() => passer.Key.from_string(key))
  .then(key => decoded.data.map(datum => key.decrypt(datum)))
}
