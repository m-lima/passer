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

import * as pack from './Pack'
import * as util from '../Util'
import Alert from '../Alert'
import Glyph from '../Glyph'
import Loading from '../Loading'

enum Status {
  Downloading,
  NotFound,
  Downloaded,
  Decrypting,
  Decrypted,
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
  <div className='dec-container dec-message'>
    <h2>Not Found</h2>
    Make sure you have the corrent link
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

  const [status, setStatus] = useState(Status.Downloading)
  const [key, setKey] = useState('')
  const [data, setData] = useState<pack.Decoded | passer.Pack[]>([])
  const { hash } = useParams()

  useEffect(() => {
    fetch(`http://localhost:3030/${hash}`, {
      redirect: 'follow',
    })
    .then(response => {
      if (response.ok) {
        return response.arrayBuffer()
      } else {
        throw Status.NotFound
      }
    })
    .then(pack.decode)
    .then(setData)
    .then(() => setStatus(Status.Downloaded))
    .catch(() => setStatus(Status.NotFound))
  }, [hash])

  const decrypt = () => {
    if (status !== Status.Downloaded) {
      return
    }

    setStatus(Status.Decrypting)

    pack.decrypt(key, data as pack.Decoded)
    .then(data => {
      setData(data)
      props.setAlerts(Alert.SUCCESS_DECRYPTING)
      setStatus(Status.Decrypted)
    })
    .catch(() => {
      props.setAlerts([Alert.INVALID_KEY])
      setStatus(Status.Downloaded)
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
    case Status.NotFound: return <NotFound />
    case Status.Downloaded: return <KeyPrompt />
    case Status.Decrypted: return <Results />
    case Status.Decrypting: return <Loading>Decrypting</Loading>
    default: case Status.Downloading: return <Loading>Downloading</Loading>
  }
}

export default Decrypt

