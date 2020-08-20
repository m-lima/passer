import './Alert.css'

import React from 'react'
import {
  Alert as BootstrapAlert,
} from 'reactstrap'

interface IProps {
  color?: string
  message: string
}

export class Message implements IProps {
  color?: string
  message: string

  constructor(message: string, color?: string) {
    this.color = color
    this.message = message
  }
}

export const TOO_SMALL = (name: string): IProps => { return { color: 'warning', message: `${name} is empty` } }
export const TOO_LARGE = (name: string): IProps => { return  { color: 'danger', message: `${name} is too big for encryption. Maximum 20 MB allowed` } }
export const ERROR_ENCRYPTING = (name: string): IProps => { return { color: 'danger', message: `${name} could not be encrypted` } }
export const ERROR_DECRYPTING = (name: string): IProps => { return { color: 'danger', message: `${name} could not be decrypted` } }
export const UNKNOWN = (name: string): IProps => { return { color: 'danger', message: `${name} processing caused an unknown error` } }

export const Banner = (props: IProps) =>
  props.message
    ? <BootstrapAlert className='Alert-banner' color={props.color}>{props.message!}</BootstrapAlert>
    : <React.Fragment />

export const Clear = (props: { clear: () => void }) => <div className='Alert-clear' onClick={() => props.clear()} >↑ CLEAR ↑</div>
