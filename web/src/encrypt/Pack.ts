import * as passer from 'passer'

import Alert from '../Alert'
import * as util from '../Util'

export const MAX_SIZE = 100 * 1024 * 1024 // 100 MiB

const generateRandom = (size: number) => {
  let array = new Uint8Array(size)
  window.crypto.getRandomValues(array)
  return array
}

const generateRandomName = () => {
  const suffix = generateRandom(8)
  return new TextDecoder().decode(suffix.map(b => b % 60).map(n => n < 10 ? n + 48 : (n < 35 ? n + 55 : n + 62)))
}

export interface Encrypted {
  hash: string
  name: string
  size: number
  data: passer.Encrypted
}

export interface Plain {
  name: string
  size: number
  data: string | File
}

export const plain = (data: string | File) => {
  if (data instanceof File) {
    return {
      name: data.name,
      size: data.size,
      data,
    }
  } else {
    return {
      name: 'Text',
      size: data.length,
      data
    }
  }
}

export const encrypt = async (pack: Plain) => {
  if (pack.size > MAX_SIZE) {
    return Alert.TOO_LARGE(pack.name)
  }

  try {
    const data = await extractData(pack)
    const encrypted = data instanceof Uint8Array
      ? key.encrypt_file(pack.name, data)
      : key.encrypt_string(pack.name, data)
    return {
      hash: generateRandomName(),
      name: pack.name,
      size: encrypted.payload().length,
      data: encrypted,
    }
  } catch {
    return Alert.ERROR_ENCRYPTING(pack.name)
  }
}

const extractData = async (pack: Plain) => {
  if (pack.data instanceof File) {
    return new Promise<Uint8Array>(resolve => {
      const reader = new FileReader()

      reader.onload = () => {
        if (reader.result) {
          resolve(new Uint8Array(reader.result as ArrayBuffer))
        }
      }

      reader.readAsArrayBuffer(pack.data as File)
    })
  } else {
    return util.yieldProcessing().then(() => pack.data as string)
  }
}

const key = new passer.Key(generateRandom(44))

export const keyString = () => key.to_string()
