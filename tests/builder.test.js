const { Jar, Builder } = require('../lib');

describe('Build client from builder', () => {
    test('w/ user agent', () => {
        let client = new Builder()
            .setUserAgent('Glub Glub')
            .build();
    });

    test('w/ jar', () => {
        let jar = new Jar();

        jar.addCookieStr('foo=bar; Domain=something', 'https://zhenyangli.me');

        let client = new Builder()
            .setJar(jar)
            .build();
    })
});
