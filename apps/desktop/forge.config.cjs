const path = require('node:path');
const { FusesPlugin } = require('@electron-forge/plugin-fuses');
const { FuseV1Options, FuseVersion } = require('@electron/fuses');

module.exports = {
  packagerConfig: {
    name: 'Gnomcode',
    executableName: 'gnomcode-desktop',
    icon: path.join(__dirname, process.platform === 'darwin'
      ? '../../web/public/icons/gnomcode.icns'
      : process.platform === 'linux'
        ? '../../web/public/icons/gnomcode-512.png'
        : '../../scripts/gnomcode.ico'),
    appBundleId: 'app.gnomcode.desktop',
    asar: true,
    extraResource: [
      path.join(__dirname, 'staging', 'bin'),
      path.join(__dirname, 'staging', 'LICENSE'),
      path.join(__dirname, 'staging', 'THIRD_PARTY_NOTICES.html'),
      path.join(__dirname, 'staging', 'DESKTOP_THIRD_PARTY_NOTICES.html'),
      path.join(__dirname, 'staging', 'ELECTRON_LICENSE.txt'),
      path.join(__dirname, 'staging', 'CHROMIUM_LICENSES.html'),
      path.join(__dirname, '../../scripts/gnomcode.ico'),
      path.join(__dirname, '../../scripts/gnomcode-tray.ico'),
      path.join(__dirname, '../../scripts/tray-badges'),
      path.join(__dirname, '../../web/public/icons/gnomcode-64.png'),
    ],
    ignore: file => Boolean(file && file !== '/package.json' && file !== '/dist' && !file.startsWith('/dist/') && file !== '/node_modules' && !file.startsWith('/node_modules/')),
  },
  makers: [
    { name: '@electron-forge/maker-squirrel', platforms: ['win32'], config: { name: 'gnomcode', authors: 'Wahaj', setupIcon: path.join(__dirname, '../../scripts/gnomcode.ico') } },
    { name: '@electron-forge/maker-zip', platforms: ['win32', 'darwin', 'linux'] },
    { name: '@electron-forge/maker-deb', platforms: ['linux'], config: { options: { productName: 'Gnomcode', icon: path.join(__dirname, '../../web/public/icons/gnomcode-512.png'), categories: ['Development'] } } },
    { name: '@electron-forge/maker-rpm', platforms: ['linux'], config: {} },
  ],
  plugins: [new FusesPlugin({
    version: FuseVersion.V1,
    [FuseV1Options.RunAsNode]: false,
    [FuseV1Options.EnableNodeOptionsEnvironmentVariable]: false,
    [FuseV1Options.EnableNodeCliInspectArguments]: false,
    [FuseV1Options.OnlyLoadAppFromAsar]: true,
  })],
};
