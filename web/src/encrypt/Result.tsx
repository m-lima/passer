import React, { useRef } from 'react'
import {
  Button,
  Input,
  InputGroup,
  InputGroupAddon,
} from 'reactstrap'

import './Result.css'

import { ReactComponent as copy } from '../img/copy-solid.svg'
import { ReactComponent as url } from '../img/link-solid.svg'
import { ReactComponent as key } from '../img/key-solid.svg'

import * as config from '../Config'
import Glyph from '../Glyph'

interface IProps {
  url: string
  keyString: string
}

const Result = (props: IProps) => {
  const singleRef = useRef<HTMLInputElement>(null)
  const copySingle = () => {
    if (singleRef && singleRef.current) {
      singleRef.current.select()
      document.execCommand('copy')
    }
  }

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
        <h6>Copy the link below and pass it to who should receive the encrypted data</h6>
        <InputGroup>
          <InputGroupAddon addonType='prepend'>
            <Button color='info' onClick={copySingle}>
              <Glyph src={copy} />
            </Button>
          </InputGroupAddon>
          <Input innerRef={singleRef} type='text' readOnly value={window.location.origin + config.Path.DECRYPT_QUICK + props.url + props.keyString} />
        </InputGroup>
      </div>

      <div className='result-block'>
        <h6>Or, for extra security, send the link and the decryption key separately</h6>
        <InputGroup>
          <InputGroupAddon addonType='prepend'>
            <Button color='info' onClick={copyUrl}>
              <Glyph src={url} />
            </Button>
          </InputGroupAddon>
          <Input innerRef={urlRef} type='text' readOnly value={window.location.origin + config.Path.DECRYPT + props.url} />
        </InputGroup>
      </div>

      <div className='result-block'>
        <InputGroup>
          <InputGroupAddon addonType='prepend'>
            <Button color='info' onClick={copyKey}>
              <Glyph src={key} />
            </Button>
          </InputGroupAddon>
          <Input innerRef={keyRef} type='text' readOnly value={props.keyString} />
        </InputGroup>
      </div>
    </>
  )
}

export default Result
