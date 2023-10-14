import React, { FunctionComponent, PropsWithChildren } from 'react'

import './Footer.scss'

const Footer: FunctionComponent<PropsWithChildren> = (props) =>
  <>
    <div className='footer-spacer' />
    <footer className='footer'>
      {props.children}
    </footer>
  </>

export default Footer
