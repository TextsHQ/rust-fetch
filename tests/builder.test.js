const { Builder } = require('../lib');

test('Builds a client w/ user agent', () => {
    let client = new Builder()
        .setUserAgent('Glub Glub')
        .build();
});
