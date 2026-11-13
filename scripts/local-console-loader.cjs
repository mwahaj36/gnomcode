'use strict';

const ts = require('../web/node_modules/typescript');

module.exports = function transpileTypeScript(source) {
  const result = ts.transpileModule(source, {
    compilerOptions: {
      jsx: ts.JsxEmit.ReactJSX,
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
      esModuleInterop: true,
      sourceMap: false,
      inlineSourceMap: false,
      removeComments: false,
    },
    fileName: this.resourcePath,
  });
  return result.outputText;
};
