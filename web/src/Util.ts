export const sizeToString = (size: number) =>
  size < 1024
    ? `${size} B`
    : size < 1024 * 1024
      ? `${(size / 1024).toFixed(1)} KiB`
      : `${(size / 1024 / 1024).toFixed(1)} MiB`

export const yieldProcessing = () => {
  return new Promise(resolve => setTimeout(resolve, 10));
}

