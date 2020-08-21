// TODO: Warning on reload (about to leave the page)
import React, { useState, useCallback } from 'react'
import {
  Button,
  Container,
  Input,
  Modal,
  ModalHeader,
  ModalFooter,
  Navbar,
  NavbarBrand,
  Progress,
} from 'reactstrap'
import { useDropzone } from 'react-dropzone'

import * as passer from 'passer'

import './App.scss'

import Alert, { Message } from './Alert'
import Pack from './Pack'

import lock from './img/lock-optimized.svg'
import { ReactComponent as Upload } from './img/file-upload-solid.svg'
import Footer from './Footer'

const generateRandom = (size: number) => {
  let array = new Uint8Array(size)
  window.crypto.getRandomValues(array)
  return array
}

const generateRandomName = () => {
  const suffix = generateRandom(6)
  return new TextDecoder().decode(suffix.map(b => b % 60).map(n => n < 10 ? n + 48 :( n < 35 ? n + 55 : n + 62)))
}

const pack = (name: string, data: string|Uint8Array) => {
  if (data.length < minSize) {
    return Message.TOO_SMALL(name)
  }

  if (data.length > maxSize) {
    return Message.TOO_LARGE(name)
  }

  return data instanceof Uint8Array ? passer.Pack.pack_file(name, data) : passer.Pack.pack_string(name, data)
}

const key = new passer.Key(generateRandom(44))

const minSize = 1
const maxSize = 20 * 1024 * 1024

const App = () => {

  const [packs, setPacks] = useState<passer.Pack[]>([])
  const [alert, setAlert] = useState<Message>()
  const [modal, setModal] = useState(false)
  const [secretText, setSecretText] = useState('')

  const toggleModal = () => setModal(!modal)

  const totalSize = () => packs.map(p => p.size()).reduce((a, b) => a + b, 0)

  const handlePacking = useCallback((value: passer.Pack | Message) => {
    if (value instanceof Message) {
      setAlert(value)
    } else {
      setPacks([...packs, value])
    }
  }, [packs])

  const packText = useCallback(() => {
    handlePacking(pack(`Message-${generateRandomName()}`, secretText))
    setSecretText('')
  }, [secretText, handlePacking])

  const packFile = useCallback((files: File[]) => {
      if (files.length !== 1) {
        setAlert(Message.ONLY_ONE_FILE)
        return
      }

      const file = files[0]
      const name = `${file.name}`
      if (file.size < minSize) {
        setAlert(Message.TOO_SMALL(name))
        return
      }

      if (file.size > maxSize) {
        setAlert(Message.TOO_LARGE(name))
        return
      }

      const reader = new FileReader()
      reader.onload = () => {
        if (reader.result) {
          handlePacking(pack(name, new Uint8Array(reader.result as ArrayBuffer)))
        }
      }
      reader.readAsArrayBuffer(file)
  }, [handlePacking])

  const {
    getRootProps,
    getInputProps,
    isDragActive,
  } = useDropzone({
    onDrop: packFile,
  })

  const sizePercentage = (totalSize() * 20 / maxSize).toFixed(1)

  return (
    <React.Fragment>
      <Navbar color='dark' dark>
        <NavbarBrand href='/'>
          <img className='d-inline-block align-top' id='lock' src={lock} alt='' />
            {' '}Passer
        </NavbarBrand>
      </Navbar>
      { alert ? <Alert {...alert} /> : '' }
      <Modal isOpen={modal} toggle={toggleModal}>
        <ModalHeader>
          Are you sure you want to clear the page?
        </ModalHeader>
        <ModalFooter>
          <Button color='success' href='/'>Clear</Button>
          <Button color='secondary' onClick={toggleModal}>Cancel</Button>
        </ModalFooter>
      </Modal>
        <Container role='main'>
          <div className='app-input'>
            <Input
              className='app-text mt-2 mb-2'
              type='textarea'
              id='secret'
              name='secret'
              placeholder={'Type message to encrypt locally on your browser'}
              autoComplete='off'
              autoFocus={true}
              onChange={e => setSecretText(e.target.value)}
              value={secretText}
              style={{ height: '100px' }}
            />
            <div className='app-dropzone' id={isDragActive ? 'active' : ''} {...getRootProps()}>
              <input {...getInputProps()} />
              Upload
              <Upload />
            </div>
          </div>
          <Button color='success' size='lg' block onClick={() => packText()}>Encrypt</Button>
          <Button color='secondary' size='lg' block onClick={toggleModal}>Clear</Button>
          { packs.map((pack, i) => <Pack
            key={i}
            plainMessage={pack.plain_message()}
            name={pack.name()}
            size={pack.size()}
          />) }
          <Progress striped value={sizePercentage}>{sizePercentage}{' %'}</Progress>
        </Container>
      <Footer>
        Copyright © {new Date().getFullYear()} Marcelo Lima | Fonts provided by <a href='https://fontawesome.com/license'>Font Awesome</a> with modifications by Marcelo Lima | Source code available on <a href='https://github.com/m-lima/passer'>GitHub</a>
      </Footer>
    </React.Fragment>
  )
}

export default App
