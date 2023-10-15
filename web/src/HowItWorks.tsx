import React, { useState } from 'react'
import {
  Button,
  ButtonGroup,
  Col,
  Row,
} from 'reactstrap'

import './HowItWorks.css'

import enc1 from './img/passer-flow-enc-1.svg'
import enc2 from './img/passer-flow-enc-2.svg'
import enc3 from './img/passer-flow-enc-3.svg'
import enc4 from './img/passer-flow-enc-4.svg'
import enc5 from './img/passer-flow-enc-5.svg'

import dec1 from './img/passer-flow-dec-1.svg'
import dec2 from './img/passer-flow-dec-2.svg'
import dec3 from './img/passer-flow-dec-3.svg'
import dec4 from './img/passer-flow-dec-4.svg'
import dec5 from './img/passer-flow-dec-5.svg'
import dec6 from './img/passer-flow-dec-6.svg'

const encryptionItems = [
  {
    src: enc1,
    header: 'Content is sent to the browser',
    caption: 'This happens locally without any upload',
  },
  {
    src: enc2,
    header: 'Content is immediately encrypted',
    caption: 'A 256-bit key is generated and used for encryption as soon as content is loaded',
  },
  {
    src: enc3,
    header: 'Encrypted content is uploaded',
    caption: 'The key stays behind in the browser and only a stream of encrypted bytes is sent',
  },
  {
    src: enc4,
    header: 'Server assigns a unique 256-bit identifier',
    caption: 'The server only knows of the identifier and which encrypted bytes it refers to'
  },
  {
    src: enc5,
    header: 'The browser returns both the identifier and the key',
    caption: 'Both are needed to access and decrypt the content. The server will delete the data after first download or if it expires',
  },
]

const decryptionItems = [
  {
    src: dec1,
    header: 'An identifier is requested',
    caption: 'The 256-bit identifier only refers to a pack of encrypted bytes in the server',
  },
  {
    src: dec2,
    header: 'The browser queries the server for the identifier',
  },
  {
    src: dec3,
    header: 'Encrypted content is downloaded',
    caption: 'The server immediately deletes the data and the browser owns the only copy',
  },
  {
    src: dec4,
    header: 'The decryption key is provided',
    caption: 'The 256-bit key is used to decrypt the data locally'
  },
  {
    src: dec5,
    header: 'The decrypted data is kept loaded in the browser',
  },
  {
    src: dec6,
    header: 'The decrypted content can be saved',
    caption: 'Being the only copy of the decrypted data, anything that is not downloaded is deleted',
  },
]

enum Page {
  ENCRYPTION,
  DECRYPTION,
}

interface Step {
  src: string,
  header: string,
  caption?: string,
}

const renderStep = (step: Step, index: number) =>
  <Row className='hiw-row' key={index}>
    <Col xl='6' lg='12' className='hiw-img'>
      <img src={step.src} alt='' />
    </Col>
    <Col className='hiw-text'>
      <h4>{step.header}</h4>
      {step.caption}
    </Col>
  </Row>

const HowItWorks = () => {

  const [page, setPage] = useState(Page.ENCRYPTION)

  const items = page === Page.DECRYPTION ? decryptionItems : encryptionItems

  return (
    <>
      <ButtonGroup style={{ width: '100%' }} size='lg'>
        <Button outline={page !== Page.ENCRYPTION} color='info' onClick={() => setPage(Page.ENCRYPTION)}>Encryption</Button>
        <Button outline={page !== Page.DECRYPTION} color='info' onClick={() => setPage(Page.DECRYPTION)}>Decryption</Button>
      </ButtonGroup>
      {items.map(renderStep)}
    </>
  )
}

export default HowItWorks
