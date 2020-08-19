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

import lock from './img/lock.svg'
import Footer from './Footer'
import * as passer from 'passer'

const encrypt = (payload: string|Uint8Array) => {
  const cipher = (typeof payload === "string") ? passer.encrypt_string(payload as string) : passer.encrypt(payload as Uint8Array)
  console.log(`Key: ${cipher.key()}`)
  console.log(`Secret: ${cipher.payload()}`)
}

const App = () => {

  const [clearModal, setClearModal] = useState(false)
  const [secretText, setSecretText] = useState('')

  const clearToggle = () => setClearModal(!clearModal)

  const onDrop = useCallback(
    (files: File[]) => {
      const reader = new FileReader()
      reader.onabort = () => console.log('file reading aborted')
      reader.onerror = () => console.log('file reading failed')
      reader.onload = () => {
        encrypt(new Uint8Array(reader.result as ArrayBuffer))
      }
      files.forEach(f => reader.readAsArrayBuffer(f))
    },
    []
  )

  const {
    getRootProps,
    getInputProps,
    fileRejections,
  } = useDropzone({
    minSize: 1,
    maxSize: 1024 * 1024 * 100,
    onDrop,
  })

  return (
    <React.Fragment>
      <Navbar color='dark' dark>
        <NavbarBrand href='/'>
          <img className='d-inline-block align-top' id='lock' src={lock} alt='' />
            {' '}Passer
        </NavbarBrand>
      </Navbar>
      <Modal isOpen={clearModal} toggle={clearToggle}>
        <ModalHeader>
          Are you sure you want to clear the page?
        </ModalHeader>
        <ModalFooter>
          <Button color='success' href='/'>Clear</Button>
          <Button color='secondary' onClick={clearToggle}>Cancel</Button>
        </ModalFooter>
      </Modal>
      <div {...getRootProps()} >
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
              <input {...getInputProps()} />
              <Button color='success' size='lg' block onClick={() => encrypt(secretText)}>Encrypt</Button>
              <Button color='secondary' size='lg' block onClick={clearToggle}>Clear</Button>
        </Container>
      </div>
      <Footer>
        Copyright © {new Date().getFullYear()} Marcelo Lima | Fonts provided by <a href='https://fontawesome.com/license'>Font Awesome</a> with modifications by Marcelo Lima | Source code available on <a href='https://githumb.com/m-lima'>GitHub</a>
      </Footer>
    </React.Fragment>
  )
}

export default App
