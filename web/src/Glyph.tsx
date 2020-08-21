import React, { FunctionComponent } from 'react'

import './Glyph.css'

interface IProps {
  src: string
}

const Glyph: FunctionComponent<IProps> = (props) =>
  <div className="glyph baseline">
    <img src={props.src} alt='' />
    { props.children }
  </div>

export default Glyph
