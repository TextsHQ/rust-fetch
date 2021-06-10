const { Jar } = require('../lib');

const { CookieJar, Cookie } = require('tough-cookie');

test('Tough cookie integration', () => {
    let toughJar = new CookieJar();

    let cookie = new Cookie({ key: 'ct0', value: 'csrfToken', secure: true, hostOnly: false, domain: 'twitter.com', maxAge: 1440 });

    toughJar.setCookie(cookie, 'https://twitter.com');

    let jar = new Jar();

    jar.useToughJar(toughJar, 'https://twitter.com');
});
