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
} from 'reactstrap'
import { useDropzone } from 'react-dropzone'

import * as passer from 'passer'

import Alert, { Message } from './Alert'

import lock from './img/lock.svg'
import Footer from './Footer'

const generateRandom = (size: number) => {
  let array = new Uint8Array(size)
  window.crypto.getRandomValues(array)
  return array
}

const encrypt = (name: string, payload: Uint8Array) => {
  if (payload.length < minSize) {
    return Message.TOO_SMALL(name)
  }

  if (payload.length > maxSize) {
    return Message.TOO_LARGE(name)
  }

  try {
    const cipher =  passer.encrypt(key, payload)
    console.log(`Key: ${key.to_string()}`)
    console.log(`Secret: ${cipher.payload()}`)
  } catch (e) {
    switch (e) {
      case 'FAILED_TO_PROCESS':
        return Message.ERROR_ENCRYPTING(name)
      case 'INVALID_KEY':
      case 'FAILED_TO_PARSE_KEY':
        return Message.UNKNOWN
    }
  }
}

const key = new passer.Key(generateRandom(44))

const minSize = 1
const maxSize = 20 * 1024 * 1024

const App = () => {

  const [alert, setAlert] = useState<Message>()
  const [modal, setModal] = useState(false)
  const [secretText, setSecretText] = useState('')

  const toggleModal = () => setModal(!modal)

  const encryptText = () => {
    setAlert(encrypt('Message', new TextEncoder().encode(secretText)))
  }

  const encryptFile = useCallback(
    (files: File[]) => {
      if (files.length !== 1) {
        setAlert(Message.ONLY_ONE_FILE)
        return
      }

      const file = files[0]
      const name = `File ${file.name}`
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
          encrypt(name, new Uint8Array(reader.result as ArrayBuffer))
        }
      }
      reader.readAsArrayBuffer(file)
  },
    []
  )

  const {
    getRootProps,
    getInputProps,
  } = useDropzone({
    onDrop: encryptFile,
  })

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
            <Input
              className='mt-2 mb-2'
              type='textarea'
              id='secret'
              name='secret'
              placeholder={'Type message or drag in files to encrypt locally on your browser'}
              autoComplete='off'
              autoFocus={true}
              onChange={e => setSecretText(e.target.value)}
              value={secretText}
              style={{ height: '10rem' }}
            />
            <Button color='success' size='lg' block onClick={() => encryptText()}>Encrypt</Button>
            <Button color='secondary' size='lg' block onClick={toggleModal}>Clear</Button>
        </Container>
      <div {...getRootProps()} >
        <input {...getInputProps()} />
          Drop here!!!
      </div>
      <Footer>
        Copyright © {new Date().getFullYear()} Marcelo Lima | Fonts provided by <a href='https://fontawesome.com/license'>Font Awesome</a> with modifications by Marcelo Lima | Source code available on <a href='https://github.com/m-lima/passer'>GitHub</a>
      </Footer>
    </React.Fragment>
  )
}

export default App
