const { Client, LogLevel } = require('../dist')

jest.setTimeout(10e3)

let client

beforeAll(() => {
  client = new Client({
    connectTimeout: 5,
    requestTimeout: 5,
    redirectLimit: 2,
    httpsOnly: true,
    https2AdaptiveWindow: true,
    logLevel: LogLevel.Debug,
    stripSensitiveHeaders: false,
  })
})

// http2 error: protocol error: unspecific protocol error detected
test('Facebook graph server', async () => {
  const ret = await client.request('https://b-graph.facebook.com', {
    headers: {
      Authorization: 'foo',
      bar: 'foo',
    },
  })

  expect(ret.statusCode).toBe(400)
})
