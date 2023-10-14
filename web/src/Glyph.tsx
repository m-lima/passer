import React, { FunctionComponent, ReactNode, SVGProps } from 'react'

import './Glyph.css'

interface IProps {
  src: string | FunctionComponent<SVGProps<SVGSVGElement>>;
  children?: ReactNode | undefined;
}

const Glyph: FunctionComponent<IProps> = (props) =>
  <div className="glyph baseline">
    {typeof props.src === 'string' ? <img src={props.src} alt='' /> : <props.src />}
    {' '}
    {props.children}
  </div>

export default Glyph
