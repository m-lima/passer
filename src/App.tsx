import React, { useState } from 'react'
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

import lock from './img/lock.svg'
import Footer from './Footer'

const App = () => {

  const [clearModal, clearSetModal] = useState(false)

  const clearToggle = () => clearSetModal(!clearModal)

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
      <Container role='main'>
            <Input
              type='textarea'
              id='secret'
              name='secret'
              placeholder={'Type message or drag in files to encrypt locally on your browser'}
              autoComplete='off'
              style={{ height: '10rem' }}
            />
                <Button color='success' size='lg' block>Encrypt</Button>
            <Button color='secondary' size='lg' block onClick={clearToggle}>Clear</Button>
      </Container>
      <Footer>
        Copyright © {new Date().getFullYear()} Marcelo Lima | Fonts provided by <a href='https://fontawesome.com/license'>Font Awesome</a> with modifications by Marcelo Lima | Source code available on <a href='https://githumb.com/m-lima'>GitHub</a>
      </Footer>
    </React.Fragment>
  )
}

export default App
