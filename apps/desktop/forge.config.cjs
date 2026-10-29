const path = require('node:path');
const { FusesPlugin } = require('@electron-forge/plugin-fuses');
const { FuseV1Options, FuseVersion } = require('@electron/fuses');

module.exports = {
  packagerConfig: {
    name: 'Spawnt',
    executableName: 'spawnt-desktop',
    icon: path.join(__dirname, '../../scripts/spawnt.ico'),
    appBundleId: 'app.spawnt.desktop',
    asar: true,
    extraResource: [
      path.join(__dirname, 'staging', 'bin'),
      path.join(__dirname, 'staging', 'EULA.txt'),
      path.join(__dirname, 'staging', 'THIRD_PARTY_NOTICES.html'),
      path.join(__dirname, 'staging', 'DESKTOP_THIRD_PARTY_NOTICES.html'),
      path.join(__dirname, 'staging', 'ELECTRON_LICENSE.txt'),
      path.join(__dirname, 'staging', 'CHROMIUM_LICENSES.html'),
      path.join(__dirname, '../../scripts/spawnt-tray.ico'),
      path.join(__dirname, '../../scripts/tray-badges'),
      path.join(__dirname, '../../web/public/icons/spawnt-64.png'),
    ],
    ignore: file => Boolean(file && file !== '/package.json' && file !== '/dist' && !file.startsWith('/dist/') && file !== '/node_modules' && !file.startsWith('/node_modules/')),
  },
  makers: [
    { name: '@electron-forge/maker-squirrel', platforms: ['win32'], config: { name: 'spawnt', authors: 'Wahaj', setupIcon: path.join(__dirname, '../../scripts/spawnt.ico') } },
    { name: '@electron-forge/maker-zip', platforms: ['win32', 'darwin', 'linux'] },
    { name: '@electron-forge/maker-deb', platforms: ['linux'], config: {} },
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
