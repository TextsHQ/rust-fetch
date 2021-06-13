const { CookieJar } = require('tough-cookie');
const { Builder } = require('../dist');

let client;

beforeAll(()  => {
    client = new Builder().build();
});

test('Fetch JSON document', async () => {
    let ret = await client.request('https://httpbin.org/json');

    expect(ret.statusCode).toBe(200);
    expect(ret.httpVersion).toBe('HTTP/2.0');
    expect(JSON.parse(ret.body)).toBeDefined();
});

describe('Compressions', () => {
    test('GZip', async () => {
        let ret = await client.request('https://httpbin.org/gzip');

        expect(ret.statusCode).toBe(200);
    });

    test('Brotli', async () => {
        let ret = await client.request('https://httpbin.org/brotli');

        expect(ret.statusCode).toBe(200);
    });
});

describe('Request methods', () => {
    let methods = ['GET', 'POST', 'PATCH', 'PUT', 'DELETE'];

    for (const method of methods) {
        test(method, async () => {
            let ret = await client.request(`https://httpbin.org/${method.toLowerCase()}`, {
                method,
            });

            expect(ret.statusCode).toBe(200);
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

    expect(ret.statusCode).toBe(200);

    let body = JSON.parse(ret.body);

    expect(body.headers.Foo).toBe('bar');
    expect(body.headers.Lemon).toBe('strawberry');
});

test('Request form', async () => {
    let ret = await client.request('https://httpbin.org/post', {
        method: 'POST',
        form: {
            foo: 'bar',
        },
    });

    expect(ret.statusCode).toBe(200);

    let body = JSON.parse(ret.body);

    expect(body.form.foo).toBe('bar');
});

test('Request cookie handling', async () => {
    let jar = new CookieJar();

    let ret = await client.request('https://httpbin.org/cookies/set', {
        cookieJar: jar,
        query: {
            foo: 'bar',
            lemon: 'juice',
            strawberry: 'blueberry',
        },
    });

    expect(ret.statusCode).toBe(302);
    expect(ret.headers['set-cookie']).toHaveLength(3);

    const cookieStr = jar.getCookieStringSync('https://httpbin.org');

    expect(cookieStr).toHaveLength(42);
});

test('Response binary data', async() => {
    let ret = await client.request('https://httpbin.org/image/webp', {
        responseType: 'BINARY',
    });

    expect(ret.statusCode).toBe(200);
    expect(ret.body.constructor.name).toBe('Buffer');
    expect(ret.body.length).toBeGreaterThan(10000);
});
