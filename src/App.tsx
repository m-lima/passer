import React, { useState, FunctionComponent } from 'react'
import {
  Button,
  Col,
  Container,
  Input,
  Modal,
  ModalHeader,
  ModalFooter,
  Navbar,
  NavbarBrand,
  Row,
} from 'reactstrap'

import lock from './img/lock.svg'
import { ReactComponent as Upload } from './img/file-upload-solid.svg'
import { ReactComponent as Remove } from './img/file-remove-solid.svg'

interface GlyphProps {
  svg: string
}

const Footer: FunctionComponent = (props) =>
  <React.Fragment>
    <div style={{ height: '100%' }} />
    <footer className='footer'>
      { props.children }
    </footer>
  </React.Fragment>

const Glyph = (props: GlyphProps) =>
  <div className="icon baseline">
    <img src={props.svg} alt='' />
  </div>

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
        <ModalHeader toggle={clearToggle}>
          Are you sure you want to clear the page?
        </ModalHeader>
        <ModalFooter>
          <Button color='success' href='/'>Clear</Button>
          <Button color='secondary' onClick={clearToggle}>Cancel</Button>
        </ModalFooter>
      </Modal>
      <Container role='main'>
        <Row>
          <Col xs='10'>
            <Input
              type='textarea'
              id='secret'
              name='secret'
              placeholder={'Type message or drag in files to encrypt locally on your browser'}
              autoComplete='off'
              style={{ height: '20rem' }}
            />
          </Col>
          <Col xs='1'>
            <Row className='SvgButton'>
              <Upload className='primary' />
            </Row>
            <Row className='SvgButton'>
              <Remove className='secondary' />
            </Row>
          </Col>
        </Row>
        <Row>
          <Col xs='10'>
            <Button color='success' size='lg' block>Encrypt</Button>
          </Col>
        </Row>
        <Row>
          <Col xs='10'>
            <Button color='secondary' size='lg' block onClick={clearToggle}>Clear</Button>
          </Col>
        </Row>
      </Container>
      <Footer>
        Copyright © {new Date().getFullYear()} Marcelo Lima | Fonts provided by <a href='https://fontawesome.com/license'>Font Awesome</a> with modifications by Marcelo Lima | Source code available on <a href='https://githumb.com/m-lima'>GitHub</a>
      </Footer>
    </React.Fragment>
  )
}

export default App
            /* <Button color='primary' size='lg' block><img src={upload} alt='' /></Button> */
            /* <Button color='primary' size='lg' block><img src={remove} alt='' /></Button> */
      /* <div style={{ flex: '1 1 auto' }} /> */
