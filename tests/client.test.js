const { Builder } = require('../lib');

let client;

beforeAll(()  => {
    client = new Builder()
        .setUserAgent('Glub Glub')
        .build();
});

test('Fetch JSON document', async () => {
    let ret = await client.request('https://httpbin.org/json');

    expect(ret.status).toBe(200);
    expect(ret.httpVersion).toBe('HTTP/2.0');
    expect(JSON.parse(ret.body)).toBeDefined();
});

describe('Compressions', () => {
    test('GZip', async () => {
        let ret = await client.request('https://httpbin.org/gzip');

        expect(ret.status).toBe(200);
    });

    test('Brotli', async () => {
        let ret = await client.request('https://httpbin.org/brotli');

        expect(ret.status).toBe(200);
    });
});

describe('Request methods', () => {
    let methods = ['GET', 'POST', 'PATCH', 'PUT', 'DELETE'];

    for (const method of methods) {
        test(method, async () => {
            let ret = await client.request(`https://httpbin.org/${method.toLowerCase()}`, {
                method,
            });

            expect(ret.status).toBe(200);
        });
    }
});

test('Request headers', async () => {
    let ret = await client.request('https://httpbin.org/headers', {
        headers: {
            foo: 'bar',
            lemon: 'strawberry',
        },
    });

    expect(ret.status).toBe(200);

    let body = JSON.parse(ret.body);

    expect(body.headers.Foo).toBe('bar');
    expect(body.headers.Lemon).toBe('strawberry');
});

test('Request User Agent', async () => {
    let ret = await client.request('https://httpbin.org/user-agent');

    expect(ret.status).toBe(200);

    let body = JSON.parse(ret.body);

    expect(body['user-agent']).toBe('Glub Glub');
});

