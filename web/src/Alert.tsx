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

  static ONLY_ONE_FILE = new Message('Only one file may be uploaded at a time', 'warning')

  static TOO_SMALL(name: string) {
    return new Message(`${name} is empty`, 'warning')
  }

  static TOO_LARGE(name: string) {
    return  new Message(`${name} is too big for encryption. Maximum 20 MB allowed`, 'danger')
  }

  static ERROR_ENCRYPTING(name: string) {
    return new Message(`${name} could not be encrypted`, 'danger')
  }

  static ERROR_DECRYPTING(name: string) {
    return new Message(`${name} could not be decrypted`, 'danger')
  }

  static UNKNOWN = new Message('An error occured. Please reload the page.', 'danger')
}

export const Alert = (props: IProps) => <BootstrapAlert style={{ margin: 0 }} color={props.color}>{props.message!}</BootstrapAlert>

export default Alert
