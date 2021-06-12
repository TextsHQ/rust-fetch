const { Jar, Builder } = require('../lib');

const { CookieJar, Cookie } = require('tough-cookie');

test('Tough cookie integration', () => {
    let toughJar = new CookieJar();

    let cookie = new Cookie({ key: 'ct0', value: 'csrfToken', secure: true, hostOnly: false, domain: 'twitter.com', maxAge: 1440 });

    toughJar.setCookie(cookie, 'https://twitter.com');

    let jar = new Jar();

    jar.useToughJar(toughJar, 'https://twitter.com');
});

test('Request w/ tough jar', async () => {
    let toughJar = new CookieJar();

    let cookie = new Cookie({ key: 'ct0', value: 'csrfToken', secure: true, hostOnly: false, domain: 'httpbin.org', maxAge: 1440 });

    toughJar.setCookie(cookie, 'https://httpbin.org');

    let jar = new Jar();

    jar.useToughJar(toughJar, 'https://httpbin.org');

    let client = new Builder()
        .setJar(jar)
        .build();

    let ret = await client.request('https://httpbin.org/cookies');

    let body= JSON.parse(ret.body);

    expect(ret.statusCode).toBe(200);
    expect(body.cookies.ct0).toBe('csrfToken');
});
