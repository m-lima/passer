import React from 'react'

import './ImageButton.css'

interface IProps {
  src: string
  color?: string
}

const deriveColor = (color?: string) =>
  color ? `ImageButton-${color}` : ''

const ImageButton = (props: IProps) =>
  <div className={`ImageButton ${deriveColor(props.color)}`} />

export default ImageButton

/* import { ReactComponent as Upload } from './img/file-upload-solid.svg' */
/* import { ReactComponent as Remove } from './img/file-remove-solid.svg' */

            /* <div className='SvgButton'> */
            /*   <Upload className='primary' /> */
            /* </div> */
            /* <div className='SvgButton'> */
            /*   <Remove className='secondary' /> */
            /* </div> */
