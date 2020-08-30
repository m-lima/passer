import React, { useRef } from 'react'
import {
  Button,
  Input,
  InputGroup,
  InputGroupAddon,
} from 'reactstrap'

import './Result.css'

import { ReactComponent as copy } from '../img/copy-solid.svg'

import * as config from '../Config'
import Glyph from '../Glyph'

interface IProps {
  url: string
  keyString: string
}

const Result = (props: IProps) => {
  const urlRef = useRef<HTMLInputElement>(null)
  const copyUrl = () => {
    if (urlRef && urlRef.current) {
      urlRef.current.select()
      document.execCommand('copy')
    }
  }

  const keyRef = useRef<HTMLInputElement>(null)
  const copyKey = () => {
    if (keyRef && keyRef.current) {
      keyRef.current.select()
      document.execCommand('copy')
    }
  }

  return (
    <>
      <div className='result-block'>
        <h6>Link</h6>
        <InputGroup>
          <InputGroupAddon addonType='prepend'>
            <Button color='info' onClick={copyUrl}>
              <Glyph src={copy} />
            </Button>
          </InputGroupAddon>
          <Input innerRef={urlRef} type='text' readOnly value={window.location.origin + config.Path.DECRYPT + props.url} />
        </InputGroup>
      </div>

      <div className='result-block'>
        <h6>Decryption key</h6>
        <InputGroup>
          <InputGroupAddon addonType='prepend'>
            <Button color='info' onClick={copyKey}>
              <Glyph src={copy} />
            </Button>
          </InputGroupAddon>
          <Input innerRef={keyRef} type='text' readOnly value={props.keyString} />
        </InputGroup>
      </div>

      <div className='result-block'>
        <Button color='success' size='lg' block href='/'>Encrypt more</Button>
      </div>
    </>
  )
}

export default Result
