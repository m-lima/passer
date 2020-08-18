import React, { FunctionComponent } from 'react'

import './Footer.scss'

const Footer: FunctionComponent = (props) =>
  <React.Fragment>
    <div style={{ height: '100%' }} />
    <footer className='footer'>
      { props.children }
    </footer>
  </React.Fragment>

export default Footer
