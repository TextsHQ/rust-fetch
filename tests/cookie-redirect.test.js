const { CookieJar } = require('tough-cookie');
const express = require('express');
const { Client } = require('../dist');

let client, server;

beforeAll(()  => {
    client = new Client({
        connectTimeout: 5,
        requestTimeout: 5,
        redirectLimit: 5,
        httpsOnly: false,
        https2AdaptiveWindow: false,
    });

    let app = express();

    app.get('/', (_req, res) => {
        res.cookie('cookie-monster', 'redirect-persist').redirect('/done');
    });

    app.get('/done', (_req, res) => {
        res.json({ ok: true });
    })

    server = app.listen(3005);
});

test('Cookie should persist after redirect', async () => {
    let jar = new CookieJar();

    let ret = await client.request('http://127.0.0.1:3005', {
        cookieJar: jar,
    });

    expect(ret.statusCode).toBe(200);

    const cookieStr = jar.getCookieStringSync('http://127.0.0.1:3000');

    expect(cookieStr).toBe('cookie-monster=redirect-persist');
});

afterAll(() => {
    server.close();
});
