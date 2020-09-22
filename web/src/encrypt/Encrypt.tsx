import React, { Dispatch, SetStateAction, useState, useRef } from 'react'
import {
  Button,
  CustomInput,
  FormGroup,
  Input,
  ListGroup,
  ListGroupItem,
  Modal,
  ModalBody,
  ModalFooter,
  Progress,
} from 'reactstrap'
import { Link } from 'react-router-dom'
import { useDropzone } from 'react-dropzone'
import { encode } from '@msgpack/msgpack'

import './Encrypt.css'

import lock from '../img/lock-solid.svg'
import { ReactComponent as trash } from '../img/trash-alt-solid.svg'
import { ReactComponent as SendFile } from '../img/file-import-solid.svg'
import { ReactComponent as SendText } from '../img/file-signature-solid.svg'

import * as config from '../Config'
import * as pack from './Pack'
import * as util from '../Util'
import Alert from '../Alert'
import Glyph from '../Glyph'
import Loading from '../Loading'
import Result from './Result'

class EncryptResult {
  alerts: Alert[]
  packs: pack.Encrypted[]

  constructor() {
    this.alerts = []
    this.packs = []
  }

  static reduce = (acc: EncryptResult, curr: Alert | pack.Encrypted) => {
    curr instanceof Alert ?  acc.alerts.push(curr) : acc.packs.push(curr)
    return acc
  }
}

class UploadResult {
  url: string
  keyString: string

  constructor(url: string, keyString: string) {
    this.url = url
    this.keyString = keyString
  }
}

interface IProps {
  setAlerts: Dispatch<SetStateAction<Alert[]>>
}

const ttlToText = (ttl: number) => {
  switch (ttl) {
    case 1: return '1 hour'
    case 2: return '12 hours'
    case 3: return '1 day'
    case 4: return '3 days'
    case 5: return '1 week'
  }
}

const ttlToQuery = (ttl: number) => {
  switch (ttl) {
    case 1: return '1h'
    case 2: return '12h'
    case 3: return '1d'
    case 4: return '3d'
    case 5: return '7d'
  }
}

const Encrypt = (props: IProps) => {

  const inputRef = useRef<HTMLInputElement>(null)
  const setInputFocus = () => {
    inputRef && inputRef.current && inputRef.current.focus()
  }

  const [loading, setLoading] = useState('')
  const [packs, setPacks] = useState<pack.Encrypted[]>([])
  const [totalSize, setTotalSize] = useState(0)
  const [modal, setModal] = useState(false)
  const [secretText, setSecretText] = useState('')
  const [ttl, setTTL] = useState(3)
  const [uploadResult, setUploadResult] = useState<UploadResult>()

  const sizePercentage = (totalSize * 100 / pack.MAX_SIZE).toFixed(1)

  const toggleModal = () => {
    setModal(!modal)
  }

  const reset = () => {
    setLoading('')
    setPacks([])
    props.setAlerts([])
  }

  const encryptPacks = (plains: pack.Plain[]) => {
    if (plains.length === 0) {
      return
    }

    setLoading('Encrypting')

    Promise.all(plains.map(pack.encrypt))
      .then(results => results.reduce(EncryptResult.reduce, new EncryptResult()))
      .then(results => {
        results.packs = [...packs, ...results.packs]

        const totalSize = results.packs.map(p => p.size).reduce((a, c) => a + c, 0)
        if (totalSize > pack.MAX_SIZE) {
          results.alerts.push(Alert.TOO_MUCH_DATA)
        }

        return { totalSize, ...results }
      })
      .then(results => {
        setPacks(results.packs)
        setTotalSize(results.totalSize)
        props.setAlerts(results.alerts)
      })
      .then(() => setLoading(''))
  }

  const encryptText = () => {
    toggleModal()
    setSecretText('')
    encryptPacks([pack.plain(secretText)])
  }

  const encryptFiles = (files: File[]) => encryptPacks(files.map(pack.plain))

  const {
    getRootProps,
    getInputProps,
    isDragActive,
  } = useDropzone({
    onDrop: encryptFiles,
  })

  const remove = (index: number) => {
    const filtered = packs.filter((_, i) => i !== index)
    setPacks(filtered)
    setTotalSize(filtered.map(p => p.size).reduce((a, c) => a + c, 0))
  }

  const send = () => {
    setLoading('Uploading')
    fetch(`${config.API}?ttl=${ttlToQuery(ttl)}`, {
      method: 'POST',
      redirect: 'follow',
      body: encode(packs.map(p => p.data.payload())),
    })
    .then(response => {
      if (response.ok) {
        return response.text()
      } else {
        throw Alert.ERROR_UPLOADING
      }
    })
    .then(url => {
      setUploadResult(new UploadResult(url, pack.keyString()))
      props.setAlerts(Alert.SUCCESS_UPLOADING)
    })
    .catch(() => props.setAlerts([Alert.ERROR_UPLOADING]))
    .then(() => setLoading(''))
  }

  const inputModal = () =>
    <Modal centered onOpened={setInputFocus} onClosed={() => setSecretText('')} isOpen={modal} toggle={toggleModal}>
      <ModalBody>
        <Input
          innerRef={inputRef}
          type='textarea'
          placeholder={'Type message to encrypt locally in your browser'}
          autoComplete='off'
          onChange={e => setSecretText(e.target.value)}
          value={secretText}
          rows={4}
        />
      </ModalBody>
      <ModalFooter>
        <Button color='success' disabled={secretText.length === 0} onClick={encryptText}>Encrypt</Button>
        <Button color='secondary' onClick={toggleModal}>Cancel</Button>
      </ModalFooter>
    </Modal>

  const packItem = (pack: pack.Encrypted, key: number) =>
    <ListGroupItem key={key}>
      <div className='spread'>
        <Glyph src={lock}>
          {`${pack.hash} (${pack.name})`}
        </Glyph>
        <span>
          {util.sizeToString(pack.size)}
          {' '}
          <span className='enc-remove' onClick={() => remove(key)}>
            <Glyph src={trash} />
          </span>
        </span>
      </div>
    </ListGroupItem>

  const packList = () =>
    <>
      <div className='enc-packs'>
        <ListGroup flush>
          {packs.map(packItem)}
        </ListGroup>
      </div>
      {Number.parseInt(sizePercentage) >= 10
        ? <Progress
            color='info'
            value={sizePercentage}
            className='enc-progress'
          >
            <span className='enc-progresss-value'>{sizePercentage}{' %'}</span>
          </Progress>
        : <></>
      }
      <FormGroup>
        <b>Expiry: {ttlToText(ttl)}</b>
        <CustomInput
          type='range'
          min={1}
          max={5}
          onChange={e => setTTL(Number.parseInt(e.target.value))}
          value={ttl}
        />
      </FormGroup>
      <Button color='success' size='lg' block onClick={send} disabled={totalSize > pack.MAX_SIZE}>Upload</Button>
      <Button color='secondary' size='lg' block onClick={reset}>Clear</Button>
    </>

  const mainContent = () =>
    <>
      {inputModal()}
      <div className='enc-input'>
        <div className='enc-input-button' id={isDragActive ? 'active' : ''} {...getRootProps()}>
          <input {...getInputProps()} />
          <SendFile style={{ paddingRight: '24px' }} className='enc-input-button-image' />
          File
        </div>
        <div className='enc-input-button' onClick={toggleModal}>
          <SendText style={{ paddingLeft: '40px' }} className='enc-input-button-image' />
          Text
        </div>
      </div>
      {packs.length > 0
        ? packList()
        : <div className='enc-instruction'>
            <span className='avoid-wrap'>Encrypt data locally in your browser</span>
            {' '}
            <span className='avoid-wrap'>and share it securely.</span>
            <br />
            <div>
              <Link to='/howitworks'>
                How it works
              </Link>
            </div>
          </div>
      }
    </>

  if (loading) {
    return <Loading>{loading}</Loading>
  } else if (uploadResult) {
    return <Result {...uploadResult} />
  } else {
    return mainContent()
  }
}

export default Encrypt
