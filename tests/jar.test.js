const { Jar } = require('../lib');

test('Create jar & append cookie string', () => {
    let jar = new Jar();

    jar.addCookieStr('foo=bar', 'https://zhenyangli.me');
});
