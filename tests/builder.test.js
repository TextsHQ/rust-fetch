const { Builder } = require('../dist');

describe('Build client from builder', () => {
    test('w/ user agent', () => {
        let client = new Builder()
            .setUserAgent('Glub Glub')
            .build();
    });
});
