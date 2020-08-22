import React from 'react'

import './Pack.css'

import file from './img/file-add-solid.svg'
import text from './img/file-remove-solid.svg'
import lock from './img/lock-solid.svg'
import Glyph from './Glyph'

interface IProps {
  plainMessage?: boolean,
  name: string,
  size: number,
}

const icon = (plainMessage?: boolean) => plainMessage === undefined ? lock : (plainMessage ? text : file)
const sizeToString = (size: number) => size < 1024 ? `${size} B` : size < 1024 * 1024 ? `${(size / 1024).toFixed(1)} KiB` : `${(size / 1024 / 1024).toFixed(1)} MiB`

const Pack = (props: IProps) =>
  <div className='pack-container'>
    <Glyph src={icon(props.plainMessage)}>
      {props.name}
    </Glyph>
    <span className='pack-size'>
      {sizeToString(props.size)}
      </span>
  </div>

export default Pack
