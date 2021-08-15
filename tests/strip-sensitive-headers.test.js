const { Client, LogLevel } = require('../dist');

jest.setTimeout(10e3);

let client;

beforeAll(()  => {
    client = new Client({
        connectTimeout: 5,
        requestTimeout: 5,
        redirectLimit: 2,
        httpsOnly: true,
        https2AdaptiveWindow: true,
        logLevel: LogLevel.Debug,
        stripSensitiveHeaders: true,
    });
});

test('Strip sensitive headers', async () => {
    let ret = await client.request('https://httpbun.com/redirect-to?url=https://httpbin.org/headers', {
        headers: {
            'Authorization': 'foo',
            'bar': 'foo',
        }
    });

    expect(ret.statusCode).toBe(200);

    let body = JSON.parse(ret.body);

    expect(body.headers.Authorization).toBeUndefined();
    expect(body.headers.Bar).toBe('foo');
});