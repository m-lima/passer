import React from 'react'

interface IProps {
  plainMessage: boolean,
  name: string,
  size: number,
}

const sizeToString = (size: number) => size < 1024 ? `${size} B` : size < 1024 * 1024 ? `${(size / 1024).toFixed(1)} KiB` : `${(size / 1024 / 1024).toFixed(1)} MiB`

const Pack = (props: IProps) => <div className='Pack'>{props.name}{' '}{sizeToString(props.size)}</div>

export default Pack
