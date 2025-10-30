import React from 'react';
import { ListGroup, ListGroupItem } from 'reactstrap';
import * as passer from 'passer_wasm';

import file from '../img/file-solid.svg';
import text from '../img/file-alt-solid.svg';

import * as util from '../Util';
import Glyph from '../Glyph';

const downloadURL = (data: string, fileName: string) => {
  const a = document.createElement('a');
  a.href = data;
  a.download = fileName;
  document.body.appendChild(a);
  a.style.display = 'none';
  a.click();
  a.remove();
};

const download = (data: Uint8Array, fileName: string) => {
  const blob = new Blob([new Uint8Array(data)], {
    type: 'application/octet-stream',
  });

  const url = window.URL.createObjectURL(blob);

  downloadURL(url, fileName);

  util.yieldProcessing().then(() => window.URL.revokeObjectURL(url));
};

export const NotFound = () => (
  <div className='dec-message'>
    <h2>Not Found</h2>
    Make sure you have the correct link and that it was not accessed before
  </div>
);

export const Corrupted = () => (
  <div className='dec-message'>
    <h2>Invalid data</h2>
    The data was downloaded but it was corrupted
  </div>
);

export const InvalidLink = () => (
  <div className='dec-message'>
    <h2>Not Found</h2>
    Make sure you have the corrent link
  </div>
);

const result = (pack: passer.Pack) =>
  pack.plain_message() ? (
    <ListGroupItem className='dec-text-block'>
      <Glyph src={text}>{pack.name()}</Glyph>
      <pre className='dec-text'>{new TextDecoder().decode(pack.data())}</pre>
    </ListGroupItem>
  ) : (
    <ListGroupItem
      className='dec-text-block'
      tag='button'
      action
      onClick={() => download(pack.data(), pack.name())}
    >
      <div className='spread'>
        <Glyph src={file}>{pack.name()}</Glyph>
        <span>{util.sizeToString(pack.size())}</span>
      </div>
    </ListGroupItem>
  );

interface ResultProps {
  data: passer.Pack[];
}

export const Results = ({ data }: ResultProps) => (
  <div className='dec-container'>
    <ListGroup flush>{(data as passer.Pack[]).map(result)}</ListGroup>
  </div>
);
