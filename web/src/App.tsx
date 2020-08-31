// TODO: Warning on reload (about to leave the page)

import React, { useState } from 'react'
import {
  Container,
  Navbar,
  NavbarBrand,
} from 'reactstrap'
import { BrowserRouter as Router, Route, Switch } from 'react-router-dom'

import './App.css'

import * as config from './Config'
import Alert, { AlertBanner } from './Alert'
import Decrypt from './decrypt/Decrypt'
import DecryptSingle from './decrypt/DecryptSingle'
import Encrypt from './encrypt/Encrypt'
import Footer from './Footer'
import HowItWorks from './HowItWorks'

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
    <Router>
      {alerts.map((alert, i) => <AlertBanner key={i} {...alert} />)}
      <Container className='app-container' role='main'>
        <Switch>
          <Route
            path={config.Path.DECRYPT + ':hash'}
            exact
            render={() => <Decrypt setAlerts={setAlerts} />}
          />
          <Route
            path={config.Path.DECRYPT_QUICK + ':hash'}
            exact
            render={() => <DecryptSingle setAlerts={setAlerts} />}
          />
          <Route
            path={config.Path.HOW_IT_WORKS}
            exact
            component={HowItWorks}
          />
          <Route
            path={config.Path.ENCRYPT}
            render={() => <Encrypt setAlerts={setAlerts} />}
          />
        </Switch>
      </Container>
    </Router>

  const footer = () =>
    <Footer>
      Copyright © {new Date().getFullYear()} Marcelo Lima | Icons provided by <a href='https://fontawesome.com/license'>Font Awesome</a> | Source code available on <a href='https://github.com/m-lima/passer'>GitHub</a>
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
