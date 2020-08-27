import React, { Dispatch, SetStateAction, useState, useRef } from 'react'
import {
  Button,
  Input,
  ListGroup,
  ListGroupItem,
  Modal,
  ModalBody,
  ModalFooter,
  Progress,
} from 'reactstrap'
import { useDropzone } from 'react-dropzone'
import { encode } from '@msgpack/msgpack'

import './Encrypt.css'

import lock from '../img/lock-solid.svg'
import { ReactComponent as SendFile } from '../img/file-import-solid.svg'
import { ReactComponent as SendText } from '../img/file-signature-solid.svg'

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

        const totalSize = results.packs.map(p => p.size).reduce((a, b) => a + b, 0)
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

  const send = () => {
    setLoading('Uploading')
    fetch('http://localhost:3030', {
      method: 'POST',
      redirect: 'follow',
      body: encode(packs.map(p => p.data.payload())),
    })
    .then(response => response.text())
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
    <ListGroupItem key={key} className='enc-list-group'>
      <div className='spread'>
        <Glyph src={lock}>
          {pack.name}
        </Glyph>
        <span>{util.sizeToString(pack.size)}</span>
      </div>
    </ListGroupItem>

  const packList = () =>
    <>
      <ListGroup flush>
        {packs.map(packItem)}
      </ListGroup>
      <Progress
        color='info'
        value={sizePercentage}
        className='enc-progress'
      >
        <span className='enc-progresss-value'>{sizePercentage}{' %'}</span>
      </Progress>
      <Button color='success' size='lg' block onClick={send} disabled={totalSize > pack.MAX_SIZE}>Done</Button>
      <Button color='secondary' size='lg' block onClick={reset}>Clear</Button>
    </>

  const mainContent = () =>
    <>
      {inputModal()}
      <div className='enc-container enc-input'>
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
            <span className='avoid-wrap'>and share it securely</span>
            {' '}
            <a href='/'>How it works</a>
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
