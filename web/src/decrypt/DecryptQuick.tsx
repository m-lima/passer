import React, { Dispatch, SetStateAction, useState, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import * as passer from 'passer_wasm';

import './Decrypt.css';

import * as config from '../Config';
import * as components from './Components';
import * as pack from './Pack';
import Alert from '../Alert';
import Loading from '../Loading';

enum Status {
  DOWNLOADING,
  INVALID_LINK,
  NOT_FOUND,
  CORRUPTED,
  DECRYPTING,
  DECRYPTED,
}

interface IProps {
  setAlerts: Dispatch<SetStateAction<Alert[]>>;
}

const Decrypt = (props: IProps) => {
  const [status, setStatus] = useState(Status.DOWNLOADING);
  const [data, setData] = useState<passer.Pack[]>([]);
  const { hash } = useParams();

  useEffect(() => {
    if (status !== Status.DOWNLOADING) {
      return;
    }

    if (!hash || hash.length !== 102) {
      setStatus(Status.INVALID_LINK);
      return;
    }

    try {
      const url = hash.substring(0, 43);
      const key = passer.Key.from_base64(hash.substring(43));

      fetch(`${config.API}${url}`, {
        redirect: 'follow',
      })
        .then(response => {
          if (response.ok) {
            return response.arrayBuffer();
          } else {
            throw Status.NOT_FOUND;
          }
        })
        .catch(() => {
          throw Status.NOT_FOUND;
        })
        .then(data => {
          try {
            setStatus(Status.DECRYPTING);
            return pack.decode(data);
          } catch {
            throw Status.CORRUPTED;
          }
        })
        .then(decoded =>
          pack.decryptWithKey(key, decoded).catch(() => {
            throw Status.CORRUPTED;
          }),
        )
        .then(data => {
          setData(data);
          props.setAlerts(Alert.SUCCESS_DECRYPTING);
          setStatus(Status.DECRYPTED);
        })
        .catch(setStatus);
    } catch {
      setStatus(Status.INVALID_LINK);
    }
  }, [status, hash, props]);

  switch (status) {
    case Status.NOT_FOUND:
      return <components.NotFound />;
    case Status.INVALID_LINK:
      return <components.InvalidLink />;
    case Status.CORRUPTED:
      return <components.Corrupted />;
    case Status.DECRYPTED:
      return <components.Results data={data} />;
    case Status.DECRYPTING:
      return <Loading>Decrypting</Loading>;
    default:
    case Status.DOWNLOADING:
      return <Loading>Downloading</Loading>;
  }
};

export default Decrypt;
