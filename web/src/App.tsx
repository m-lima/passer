// TODO: Warning on reload (about to leave the page)
// TODO: Fix Alert.tsx

import React, { useState, useRef } from 'react'
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
import { ReactComponent as Write } from './img/edit-solid.svg'
import { ReactComponent as Upload } from './img/file-upload-solid.svg'

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

const key = new passer.Key(generateRandom(44))

const minSize = 1
const maxSize = 20 * 1024 * 1024

const App = () => {

  const inputRef = useRef<HTMLInputElement>(null)
  const setInputFocus = () => {
    inputRef && inputRef.current && inputRef.current.focus()
  }

  const [packs, setPacks] = useState<EncryptedPack[]>([])
  const [alert, setAlert] = useState<Message>()
  const [modal, setModal] = useState(false)
  const [secretText, setSecretText] = useState('')
  const [encrypting, setEncrypting] = useState(false)

  const toggleModal = () => {
    setModal(!modal)
  }

  const pack = (name: string, data: string|Uint8Array) => {
    setAlert(undefined)

    if (data.length < minSize) {
      setAlert(Message.TOO_SMALL(name))
    } else if (data.length > maxSize) {
      setAlert(Message.TOO_LARGE(name))
    } else {
      setEncrypting(true)
      new Promise<EncryptedPack>(resolve => setTimeout(() => resolve(
        new EncryptedPack(name, data instanceof Uint8Array
            ? key.encrypt_file(name, data)
            : key.encrypt_string(name, data))), 10))
        .then(pack => setPacks([...packs, pack]))
        .catch(() => setAlert(Message.UNKNOWN))
        .then(() => setEncrypting(false))
    }
  }

  const packText = () => {
    setModal(false)
    pack('Message', secretText)
  }

  const packFile = (files: File[]) => {
    if (files.length !== 1) {
      setAlert(Message.ONLY_ONE_FILE)
      return
    }

    const file = files[0]
    const name = `${file.name}`

    if (file.size < minSize) {
      setAlert(Message.TOO_SMALL(name))
    } else if (file.size > maxSize) {
      setAlert(Message.TOO_LARGE(name))
      return
    } else {
      const reader = new FileReader()
      reader.onload = () => {
        if (reader.result) {
          pack(name, new Uint8Array(reader.result as ArrayBuffer))
        }
      }
      reader.readAsArrayBuffer(file)
    }
  }

  const {
    getRootProps,
    getInputProps,
    isDragActive,
  } = useDropzone({
    onDrop: packFile,
  })

  const totalSize = () => packs.map(p => p.size).reduce((a, b) => a + b, 0)
  const sizePercentage = (totalSize() * 20 / maxSize).toFixed(1)

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
          placeholder={'Type message to encrypt locally on your browser'}
          autoComplete='off'
          onChange={e => setSecretText(e.target.value)}
          value={secretText}
          rows={4}
        />
      </ModalBody>
      <ModalFooter>
        <Button color='success' onClick={packText}>Encrypt</Button>
        <Button color='secondary' onClick={toggleModal}>Cancel</Button>
      </ModalFooter>
    </Modal>

  const packList = () =>
    <>
      <ListGroup flush>
        { packs.map((pack, i) => <ListGroupItem key={i}><Pack name={pack.name} size={pack.size} /></ListGroupItem>) }
      </ListGroup>
      <Progress color='info' value={sizePercentage}>{sizePercentage}{' %'}</Progress>
      <Button color='success' size='lg' block onClick={() => packText()}>Done</Button>
      <Button color='secondary' size='lg' block onClick={() => setPacks([])}>Clear</Button>
    </>

      const spinner = () => <div className='app-spinner'><Spinner className='spinner' color="info" /></div>

  const mainContent = () =>
    <>
      <div className='app-input'>
        <div className='app-input-button' id={isDragActive ? 'active' : ''} {...getRootProps()}>
          <input {...getInputProps()} />
          <Upload className='app-input-button-image' />
          Upload
        </div>
        <div className='app-input-button' onClick={toggleModal}>
          <Write style={{ paddingLeft: '16px' }} className='app-input-button-image' />
          Message
        </div>
      </div>
      {packs.length > 0 ? packList() : ''}
    </>

  const footer = () =>
    <Footer>
      Copyright © {new Date().getFullYear()} Marcelo Lima | Fonts provided by <a href='https://fontawesome.com/license'>Font Awesome</a> | Source code available on <a href='https://github.com/m-lima/passer'>GitHub</a>
    </Footer>

  return (
    <>
      {navBar()}
      {inputModal()}
      {alert ? <Alert {...alert} /> : ''}
      <Container role='main'>
        {encrypting ? spinner() : mainContent()}
      </Container>
      {footer()}
    </>
  )
}

export default App
