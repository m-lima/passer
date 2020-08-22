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

  static TOO_LARGE(name: string) {
    return  new Message(`${name} is too big for encryption. Maximum 20 MB allowed.`, 'warning')
  }

  static ERROR_ENCRYPTING(name: string) {
    return new Message(`${name} could not be encrypted.`, 'warning')
  }

  static ERROR_DECRYPTING(name: string) {
    return new Message(`${name} could not be decrypted.`, 'warning')
  }

  static TOO_MUCH_DATA = new Message('Too much data encrypted. Maximum 100 MB allowed.', 'danger')
  static UNKNOWN = new Message('An error occured. Please reload the page.', 'danger')
}

export const Alert = (props: IProps) => <BootstrapAlert style={{ margin: 0 }} color={props.color}>{props.message!}</BootstrapAlert>

export default Alert
