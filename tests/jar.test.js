const { Jar, Client, Builder } = require('../lib');

test('Create jar & append cookie string', () => {
    let jar = new Jar();

    jar.addCookieStr('foo=bar', 'https://zhenyangli.me');
});

test('Request w/ own jar', async () => {
    let jar = new Jar()
        .addCookieStr('foo=bar', 'https://httpbin.org/');

    let client = new Builder()
        .setJar(jar)
        .build();

    let ret = await client.request('https://httpbin.org/cookies');

    let json = JSON.parse(ret.body);

    expect(ret.statusCode).toBe(200);
    expect(json.cookies.foo).toBe('bar');
});
