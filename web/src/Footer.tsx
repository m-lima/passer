import React, { FunctionComponent } from 'react'

import './Footer.scss'

const Footer: FunctionComponent = (props) =>
  <>
    <div className='footer-spacer' />
    <footer className='footer'>
      { props.children }
    </footer>
  </>

export default Footer
