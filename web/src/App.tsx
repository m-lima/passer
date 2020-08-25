// TODO: Warning on reload (about to leave the page)
// TODO: Click to dismiss alert

import React, { useState } from 'react'
import {
  Container,
  Navbar,
  NavbarBrand,
} from 'reactstrap'

import './App.css'

import Alert, { AlertBanner } from './Alert'
import Footer from './Footer'
import Encrypt from './encrypt/Encrypt'

import lock from './img/lock-optimized.svg'

const App = () => {

  const [alerts, setAlerts] = useState<Alert[]>([])

  const navBar = () =>
    <Navbar color='dark' dark>
      <Container>
        <NavbarBrand href='/'>
          <img className='d-inline-block align-top' id='lock' src={lock} alt='' />
          {' '}
          Passer
        </NavbarBrand>
      </Container>
    </Navbar>

  const mainContent = () =>
    <>
      {alerts.map((alert, i) => <AlertBanner key={i} {...alert} />)}
      <Container role='main'>
        <Encrypt setAlerts={setAlerts} />
      </Container>
    </>

  const footer = () =>
    <Footer>
      Copyright © {new Date().getFullYear()} Marcelo Lima | Fonts provided by <a href='https://fontawesome.com/license'>Font Awesome</a> | Source code available on <a href='https://github.com/m-lima/passer'>GitHub</a>
    </Footer>

  return (
    <>
      {navBar()}
      {mainContent()}
      {footer()}
    </>
  )
}

export default App
