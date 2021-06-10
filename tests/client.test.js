const { Builder } = require('../lib');

test('Fetch JSON document', async () => {
    let client = new Builder()
        .setUserAgent('Glub Glub')
        .build();

    let ret = await client.request('https://httpbin.org/json');

    expect(ret.status).toBe(200);
    expect(ret.httpVersion).toBe('HTTP/2.0');
    expect(JSON.parse(ret.data)).toBeDefined();
});
