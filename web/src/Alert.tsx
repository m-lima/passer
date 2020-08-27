import React from 'react'
import {
  Alert as BootstrapAlert,
  Container,
} from 'reactstrap'

interface IProps {
  color?: string
  message: string
}

export class Alert implements IProps {
  color?: string
  message: string

  constructor(message: string, color?: string) {
    this.color = color
    this.message = message
  }

  static TOO_LARGE = (name: string) => new Alert(`${name} is too big for encryption. Maximum 100 MB allowed.`, 'warning')
  static TOO_MUCH_DATA = new Alert('Too much data encrypted. Maximum 100 MB allowed.', 'danger')
  static ERROR_ENCRYPTING = (name: string) => new Alert(`${name} could not be encrypted.`, 'warning')
  static ERROR_UPLOADING = new Alert('A problem occurred trying to upload.', 'danger')
  static SUCCESS_UPLOADING = [new Alert('Secret successfully uploaded.', 'success'), new Alert('The generated link can only be downloaded once so don\'t open it yourself.', 'info')]

  static INVALID_KEY = new Alert('The encrypted key is invalid.', 'warning')
  static SUCCESS_DECRYPTING = [new Alert('Secret successfully decrypted.', 'success'), new Alert('This page can only be opened once so don\'t forget to download all the files you need.', 'info')]

  static UNKNOWN = new Alert('An error occured. Please reload the page.', 'danger')
}

export const AlertBanner = (props: IProps) =>
  <BootstrapAlert style={{ margin: 0 }} color={props.color}>
    <Container>
      {props.message!}
    </Container>
  </BootstrapAlert>

export default Alert
