import React, {FunctionComponent } from 'react'
import { Spinner } from 'reactstrap'

import './Loading.css'

const Loading: FunctionComponent = (props) =>
  <div className='loading'>
    <div className='loading-container'>
      <Spinner className='loading-spinner' color="info" />
        <div className='loading-text'>
          { props.children }
      </div>
    </div>
  </div>

export default Loading
