module.exports = config => {
  const wasmExtensionRegExp = /\.wasm$/;

  config.experiments = {
    layers: true,
    asyncWebAssembly: true,
  };

  config.module.rules.forEach(rule => {
    (rule.oneOf || []).forEach(oneOf => {
      if (oneOf.type === "asset/resource") {
        oneOf.exclude.push(wasmExtensionRegExp)
      }
    })
  })

  return config
}
