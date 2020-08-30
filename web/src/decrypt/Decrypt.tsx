import React, { Dispatch, SetStateAction, useState, useEffect } from 'react'
import {
  Button,
  Input,
  InputGroup,
  InputGroupAddon,
  InputGroupText,
  ListGroup,
  ListGroupItem,
} from 'reactstrap'
import { useParams } from 'react-router-dom'
import * as passer from 'passer'

import './Decrypt.css'

import keyImg from '../img/key-solid.svg'
import lock from '../img/lock-solid.svg'
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
  NOT_FOUND,
  CORRUPTED,
  DOWNLOADED,
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

const NotFound = () =>
  <div className='dec-message'>
    <h2>Not Found</h2>
    Make sure you have the corrent link and that it was not accessed before
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
  const [key, setKey] = useState('')
  const [data, setData] = useState<pack.Decoded | passer.Pack[]>([])
  const { hash } = useParams()

  useEffect(() => {
    fetch(`${config.API}${hash}`, {
      redirect: 'follow',
    })
    .then(response => response.arrayBuffer())
    .catch(() => { throw Status.NOT_FOUND })
    .then(data => { try { return pack.decode(data) } catch { throw Status.CORRUPTED } })
    .then(setData)
    .then(() => setStatus(Status.DOWNLOADED))
    .catch(setStatus)
  }, [hash])

  const decrypt = () => {
    if (status !== Status.DOWNLOADED) {
      return
    }

    setStatus(Status.DECRYPTING)

    pack.decrypt(key, data as pack.Decoded)
    .then(data => {
      setData(data)
      props.setAlerts(Alert.SUCCESS_DECRYPTING)
      setStatus(Status.DECRYPTED)
    })
    .catch(() => {
      props.setAlerts([Alert.INVALID_KEY])
      setStatus(Status.DOWNLOADED)
    })
  }

  const KeyPrompt = () =>
    <div className='dec-container'>
      <ListGroup flush>
        <ListGroupItem>
          <div className='spread'>
            <span>
              <Glyph src={lock}>
                {hash}
              </Glyph>
            </span>
            <span>
              {util.sizeToString(data.length)}
            </span>
          </div>
        </ListGroupItem>
      </ListGroup>
      <InputGroup>
        <InputGroupAddon addonType='prepend'>
          <InputGroupText>
            <Glyph src={keyImg} />
          </InputGroupText>
        </InputGroupAddon>
        <Input
          type='text'
          autoFocus
          placeholder={'Insert the decryption key shared with you'}
          autoComplete='off'
          onChange={e => setKey(e.target.value)}
          value={key}
        />
        <InputGroupAddon addonType='append'>
          <Button color='success' onClick={decrypt} disabled={key.length !== 60}>
            Decrypt
          </Button>
        </InputGroupAddon>
      </InputGroup>
    </div>

  const Results = () =>
    <div className='dec-container'>
      <ListGroup flush>
        {(data as passer.Pack[]).map(result)}
      </ListGroup>
    </div>

  switch (status) {
    case Status.NOT_FOUND: return <NotFound />
    case Status.CORRUPTED: return <Corrupted />
    case Status.DOWNLOADED: return <KeyPrompt />
    case Status.DECRYPTED: return <Results />
    case Status.DECRYPTING: return <Loading>Decrypting</Loading>
    default: case Status.DOWNLOADING: return <Loading>Downloading</Loading>
  }
}

export default Decrypt

