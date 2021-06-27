const FormData = require('form-data');
const { CookieJar } = require('tough-cookie');
const { Client } = require('../dist');

let client;

beforeAll(()  => {
    client = new Client({
        connectTimeout: 5,
        requestTimeout: 5,
        redirectLimit: 0,
        httpsOnly: true,
        https2AdaptiveWindow: true,
    });
});

test('Object value should error', async () => {
    await expect(client.request('https://httpbin.org/anything', {
        headers: {
            foo: {}
        },
    }))
    .rejects
    .toThrow('Object cannot be passed as a value');
});
