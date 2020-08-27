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
import { decode } from '@msgpack/msgpack'
import * as passer from 'passer'

import './Decrypt.css'

import keyImg from '../img/key-solid.svg'
import lock from '../img/lock-solid.svg'

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

const NotFound = () =>
  <div className='dec-container dec-message'>
    <h2>Not Found</h2>
    Make sure you have the corrent link
  </div>

interface IProps {
  setAlerts: Dispatch<SetStateAction<Alert[]>>
}

const Decrypt = (props: IProps) => {

  const [status, setStatus] = useState(Status.Downloading)
  const [key, setKey] = useState('')
  const [data, setData] = useState<Uint8Array[] | passer.Pack[]>([])
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
    .then(payload => decode(payload) as Uint8Array[])
    .then(setData)
    .then(() => setStatus(Status.Downloaded))
    .catch(() => setStatus(Status.NotFound))
  }, [hash])

  const decrypt = () => {
    if (status !== Status.Downloaded) {
      return
    }

    setStatus(Status.Decrypting)

    var cipher
    try {
      const cipher = passer.Key.from_string(key)
      setData((data as Uint8Array[]).map(datum => cipher.decrypt(datum)))
      setStatus(Status.Decrypted)
    } catch (e) {
      console.log(e)
      props.setAlerts([Alert.INVALID_KEY])
      setStatus(Status.Downloaded)
    }
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

  switch (status) {
    case Status.Downloading: return <Loading>Downloading</Loading>
    case Status.NotFound: return <NotFound />
    case Status.Downloaded: return <KeyPrompt />
    default: return <KeyPrompt />
  }
}

export default Decrypt

