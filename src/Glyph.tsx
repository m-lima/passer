import React from 'react'

import './Glyph.css'

interface GlyphProps {
  src: string
}

const Glyph = (props: GlyphProps) =>
  <div className="Glyph baseline">
    <img src={props.src} alt='' />
    bla
  </div>

export default Glyph
