import React, {useEffect} from 'react'

interface IProps {
  work?: () => void
}

const Worker = ({ work }: IProps) => {
  useEffect(() => work && work(), [work])

  return <>{work && 'Loading'}</>
}

export default Worker
