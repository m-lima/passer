import React, { Dispatch, SetStateAction, useState, useEffect } from 'react'
import {
  Button,
  Input,
  InputGroup,
  InputGroupText,
  ListGroup,
  ListGroupItem,
} from 'reactstrap'
import { useParams } from 'react-router-dom'
import * as passer from 'passer'

import './Decrypt.css'

import keyImg from '../img/key-solid.svg'
import lock from '../img/lock-solid.svg'

import * as config from '../Config'
import * as components from './Components'
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

interface IProps {
  setAlerts: Dispatch<SetStateAction<Alert[]>>
}

const DecryptStepped = (props: IProps) => {

  const [status, setStatus] = useState(Status.DOWNLOADING)
  const [key, setKey] = useState('')
  const [data, setData] = useState<pack.Decoded | passer.Pack[]>([])
  const { hash } = useParams()

  useEffect(() => {
    fetch(`${config.API}${hash}`, {
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
      .then(data => { try { return pack.decode(data) } catch { throw Status.CORRUPTED } })
      .then(setData)
      .then(() => setStatus(Status.DOWNLOADED))
      .catch(setStatus)
  }, [hash])

  const decrypt = () => {
    if (!data || !key || status !== Status.DOWNLOADED) {
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
        <InputGroupText>
          <Glyph src={keyImg} />
        </InputGroupText>
        <Input
          type='text'
          autoFocus
          placeholder={'Insert the decryption key shared with you'}
          autoComplete='off'
          onChange={e => setKey(e.target.value)}
          value={key}
        />
        <Button color='success' onClick={decrypt} disabled={key.length !== 59}>
          Decrypt
        </Button>
      </InputGroup>
    </div>

  switch (status) {
    case Status.NOT_FOUND: return <components.NotFound />
    case Status.CORRUPTED: return <components.Corrupted />
    case Status.DOWNLOADED: return <KeyPrompt />
    case Status.DECRYPTED: return <components.Results data={data as passer.Pack[]} />
    case Status.DECRYPTING: return <Loading>Decrypting</Loading>
    default: case Status.DOWNLOADING: return <Loading>Downloading</Loading>
  }
}

export default DecryptStepped

