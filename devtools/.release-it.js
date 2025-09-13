const path = require('path');
const fs = require('fs');

const changelogHeaderTemplate = fs.readFileSync(
  path.join(__dirname, './template/changelog/header.hbs'),
  'utf-8',
);

module.exports = {
  git: {
    commitMessage: 'chore: release v${version}',
  },
  npm: {
    publish: false,
  },
  github: {
    release: true,
    releaseName: 'v${version}',
    assets: ['./voicebot_*.zip'],
  },
  plugins: {
    '@release-it/conventional-changelog': {
      preset: 'conventionalcommits',
      writerOpts: {
        headerPartial: changelogHeaderTemplate,
      },
    },
  },
  hooks: {
    'after:bump':
      "sed -i 's/voicebot:${latestVersion}/voicebot:${version}/g' ../deployment/docker-compose.yml",
    'before:git:release': 'git add ../deployment/docker-compose.yml',
    'before:github:release':
      "cp -r ../deployment ./voicebot && zip -r 'voicebot_${version}.zip' ./voicebot && rm -rf ./voicebot",
  },
};
