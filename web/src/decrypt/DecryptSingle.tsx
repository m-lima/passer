import React, { Dispatch, SetStateAction, useState, useEffect } from 'react'
import {
  ListGroup,
  ListGroupItem,
} from 'reactstrap'
import { useParams } from 'react-router-dom'
import * as passer from 'passer'

import './Decrypt.css'

import file from '../img/file-solid.svg'
import text from '../img/file-alt-solid.svg'

import * as config from '../Config'
import * as pack from './Pack'
import * as util from '../Util'
import Alert from '../Alert'
import Glyph from '../Glyph'
import Loading from '../Loading'

enum Status {
  DOWNLOADING,
  INVALID_LINK,
  NOT_FOUND,
  CORRUPTED,
  DECRYPTING,
  DECRYPTED,
}

const downloadURL = (data: string, fileName: string) => {
  const a = document.createElement('a')
  a.href = data
  a.download = fileName
  document.body.appendChild(a)
  a.style.display = 'none'
  a.click()
  a.remove()
}

const download = (data: Uint8Array, fileName: string) => {
  const blob = new Blob([data], {
    type: 'application/octet-stream'
  })

  const url = window.URL.createObjectURL(blob)

  downloadURL(url, fileName)

  util.yieldProcessing().then(() => window.URL.revokeObjectURL(url))
}

const InvalidLink = () =>
  <div className='dec-message'>
    <h2>Not Found</h2>
    Make sure you have the corrent link
  </div>

const NotFound = () =>
  <div className='dec-message'>
    <h2>Not Found</h2>
    Make sure you have the correct link and that it was not accessed before
  </div>

const Corrupted = () =>
  <div className='dec-message'>
    <h2>Invalid data</h2>
    The data was downloaded but it was corrupted
  </div>

const result = (pack: passer.Pack, index: number) =>
  pack.plain_message()
    ? <ListGroupItem className='dec-text-block'>
        <Glyph src={text}>{pack.name()}</Glyph>
        <pre className='dec-text'>
          {new TextDecoder().decode(pack.data())}
        </pre>
      </ListGroupItem>
    : <ListGroupItem className='dec-text-block' tag='button' action onClick={() => download(pack.data(), pack.name())}>
        <div className='spread'>
          <Glyph src={file}>{pack.name()}</Glyph>
          <span>{util.sizeToString(pack.size())}</span>
        </div>
      </ListGroupItem>

interface IProps {
  setAlerts: Dispatch<SetStateAction<Alert[]>>
}

const Decrypt = (props: IProps) => {

  const [status, setStatus] = useState(Status.DOWNLOADING)
  const [data, setData] = useState<passer.Pack[]>([])
  const { hash } = useParams()

  useEffect(() => {
    if (status !== Status.DOWNLOADING) {
      return
    }

    if (hash.length !== 102) {
      setStatus(Status.INVALID_LINK)
      return
    }

    try {
      const url = hash.substr(0, 43)
      const key = passer.Key.from_string(hash.substr(43))

      fetch(`${config.API}${url}`, {
        redirect: 'follow',
      })
      .then(response => {
        if (response.ok) {
          return response.arrayBuffer()
        } else {
          throw Status.NOT_FOUND
        }
      })
      .catch(() => { throw Status.NOT_FOUND })
      .then(data => {
        try {
          setStatus(Status.DECRYPTING)
          return pack.decode(data)
        } catch {
          throw Status.CORRUPTED
        }
      })
      .then(decoded => pack.decryptWithKey(key, decoded).catch(() => { throw Status.CORRUPTED }))
      .then(data => {
          setData(data)
          props.setAlerts(Alert.SUCCESS_DECRYPTING)
          setStatus(Status.DECRYPTED)
      })
      .catch(setStatus)
    } catch {
      setStatus(Status.INVALID_LINK)
    }
  }, [status, hash, props])

  const Results = () =>
    <div className='dec-container'>
      <ListGroup flush>
        {(data).map(result)}
      </ListGroup>
    </div>

  switch (status) {
    case Status.NOT_FOUND: return <NotFound />
    case Status.INVALID_LINK: return <InvalidLink />
    case Status.CORRUPTED: return <Corrupted />
    case Status.DECRYPTED: return <Results />
    case Status.DECRYPTING: return <Loading>Decrypting</Loading>
    default: case Status.DOWNLOADING: return <Loading>Downloading</Loading>
  }
}

export default Decrypt

