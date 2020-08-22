// TODO: Warning on reload (about to leave the page)
// TODO: Click to dismiss alert

import React, { useState, useEffect, useRef } from 'react'
import {
  Button,
  Container,
  Input,
  ListGroup,
  ListGroupItem,
  Modal,
  ModalBody,
  ModalFooter,
  Navbar,
  NavbarBrand,
  Progress,
  Spinner,
} from 'reactstrap'
import { useDropzone } from 'react-dropzone'

import * as passer from 'passer'

import './App.css'

import Alert, { Message } from './Alert'
import Pack from './Pack'
import Footer from './Footer'

import lock from './img/lock-optimized.svg'
import { ReactComponent as SendFile } from './img/file-import-solid.svg'
import { ReactComponent as SendText } from './img/file-signature-solid.svg'

class EncryptedPack {
  name: string
  size: number
  data: passer.Encrypted

  constructor(name: string, data: passer.Encrypted) {
    this.name = `${generateRandomName()} (${name})`
    this.size = data.payload().length
    this.data = data
  }
}

const generateRandom = (size: number) => {
  let array = new Uint8Array(size)
  window.crypto.getRandomValues(array)
  return array
}

const generateRandomName = () => {
  const suffix = generateRandom(8)
  return new TextDecoder().decode(suffix.map(b => b % 60).map(n => n < 10 ? n + 48 :( n < 35 ? n + 55 : n + 62)))
}

const render = () => {
  return new Promise(resolve => setTimeout(resolve, 10));
}

const pack = async (name: string, data: string|Uint8Array) => {
  if (data.length > maxSize) {
    return Message.TOO_LARGE(name)
  }

  await render()

  try {
    return new EncryptedPack(name, data instanceof Uint8Array
      ? key.encrypt_file(name, data)
      : key.encrypt_string(name, data))
  } catch {
    return Message.ERROR_ENCRYPTING(name)
  }
}

const key = new passer.Key(generateRandom(44))

const maxSize = 20 * 1024 * 1024

const App = () => {

  const inputRef = useRef<HTMLInputElement>(null)
  const setInputFocus = () => {
    inputRef && inputRef.current && inputRef.current.focus()
  }

  const [packs, setPacks] = useState<EncryptedPack[]>([])
  const [alerts, setAlerts] = useState<Message[]>([])
  const [modal, setModal] = useState(false)
  const [secretText, setSecretText] = useState('')
  const [encrypting, setEncrypting] = useState(false)
  const [totalSize, setTotalSize] = useState(0)

  useEffect(() => {
    setTotalSize(packs.map(p => p.size).reduce((a, b) => a + b, 0))
  }, [packs])

  useEffect(() => {
    if (totalSize > maxSize * 5 && alerts[alerts.length - 1] !== Message.TOO_MUCH_DATA) {
      setAlerts([...alerts, Message.TOO_MUCH_DATA])
    }
  }, [totalSize, alerts])

  const sizePercentage = (totalSize * 20 / maxSize).toFixed(1)

  const toggleModal = () => {
    setModal(!modal)
  }

  const packText = () => {
    setModal(false)
    setEncrypting(true)
    setAlerts([])
    Promise.resolve(pack('Message', secretText))
      .then(r => r instanceof Message ? setAlerts([r]) : setPacks([...packs, r]))
      .then(() => setEncrypting(false))
  }

  const packFile = (files: File[]) => {
    setEncrypting(true)
    setAlerts([])

    const readFile = (file: File): Promise<Message | EncryptedPack> => {
      const name = `${file.name}`

      if (file.size > maxSize) {
        return Promise.resolve(Message.TOO_LARGE(name))
      }

      return new Promise(resolve => {
        const reader = new FileReader()

        reader.onload = () => {
          if (reader.result) {
            resolve(pack(name, new Uint8Array(reader.result as ArrayBuffer)))
          }
        }

        reader.readAsArrayBuffer(file)
      })
    }

    interface Acc {
      messages: Message[]
      packs: EncryptedPack[]
    }

    const emptyAcc = () => { return {
      messages: [],
      packs: [],
    }}

    const accumulate = (acc: Acc, curr: EncryptedPack | Message) => {
      curr instanceof EncryptedPack ? acc.packs.push(curr) : acc.messages.push(curr)
      return acc
    }

    Promise.all(files.map(readFile))
      .then(r => r.reduce(accumulate, emptyAcc()))
      .then(r => {
        setPacks([...packs, ...r.packs])
        setAlerts(r.messages)
      })
      .then(() => setEncrypting(false))
  }

  const {
    getRootProps,
    getInputProps,
    isDragActive,
  } = useDropzone({
    onDrop: packFile,
  })

  const navBar = () =>
    <Navbar color='dark' dark>
      <NavbarBrand href='/'>
        <img className='d-inline-block align-top' id='lock' src={lock} alt='' />
        {' '}
        Passer
      </NavbarBrand>
    </Navbar>

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
        <Button color='success' onClick={packText} disabled={secretText.length === 0}>Encrypt</Button>
        <Button color='secondary' onClick={toggleModal}>Cancel</Button>
      </ModalFooter>
    </Modal>

  const packList = () =>
    <>
      <ListGroup flush>
        { packs.map((pack, i) => <ListGroupItem key={i}><Pack name={pack.name} size={pack.size} /></ListGroupItem>) }
      </ListGroup>
      <Progress color='info' value={sizePercentage}>{sizePercentage}{' %'}</Progress>
      <Button color='success' size='lg' block onClick={() => packText()} disabled={totalSize > maxSize * 5}>Done</Button>
      <Button color='secondary' size='lg' block onClick={() => setPacks([])}>Clear</Button>
    </>

      const spinner = () => <div className='app-spinner'><Spinner className='spinner' color="info" /></div>

  const mainContent = () =>
    <Container role='main'>
      <div className='app-input'>
        <div className='app-input-button' id={isDragActive ? 'active' : ''} {...getRootProps()}>
          <input {...getInputProps()} />
          <SendFile style={{ paddingRight: '24px' }} className='app-input-button-image' />
          File
        </div>
        <div className='app-input-button' onClick={toggleModal}>
          <SendText style={{ paddingLeft: '40px' }} className='app-input-button-image' />
          Text
        </div>
      </div>
      {packs.length > 0 ? packList() : ''}
    </Container>

  const footer = () =>
    <Footer>
      Copyright © {new Date().getFullYear()} Marcelo Lima | Fonts provided by <a href='https://fontawesome.com/license'>Font Awesome</a> | Source code available on <a href='https://github.com/m-lima/passer'>GitHub</a>
    </Footer>

  return (
    <>
      {navBar()}
      {inputModal()}
      {alerts.map(alert => <Alert {...alert} />)}
      {encrypting ? spinner() : mainContent()}
      {packs.length > 0 
        ? <></>
        : <div className='app-instruction'>
            <span className='avoid-wrap'>Encrypt data locally in your browser</span>
            {' '}
            <span className='avoid-wrap'>and share it securely</span>
          </div>
      }
      {footer()}
    </>
  )
}

export default App
