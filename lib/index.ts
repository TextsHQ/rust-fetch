import { promisify } from 'util'
import * as FormData from 'form-data'
import { CookieJar } from 'tough-cookie'

const {
  clientRequest,

  builderNew,
  builderConnectTimeout,
  builderRequestTimeout,
  builderRedirectLimit,
  builderHttpsOnly,
  builderStripSensitiveHeaders,
  builderHttps2AdaptiveWindow,
  builderProxy,
  builderLogLevel,
  builderBuild,
} = require('../rf.node')

const requestPromise = promisify(clientRequest)

export interface ClientOptions {
  /**
     * Timeout in seconds for the connection phase.
     */
  connectTimeout?: number

  /**
     * Timeout in seconds from start connecting to response body finished.
     */
  requestTimeout?: number

  /**
     * Maximum redirects allowed.
     *
     * A limit of 0 for no redirect allowed.
     */
  redirectLimit?: number

  /**
     * Https only
     */
  httpsOnly?: boolean

  /**
     * Whether to strip sensitive headers such as authorization, etc.
     *
     * Defaults to false.
     */
  stripSensitiveHeaders?: boolean

  /**
     * Use adaptive window size for https2
     */
  https2AdaptiveWindow?: boolean

  /**
     * Proxy URL.
     *
     * Http(s) and socks5 proxies are supported.
     */
  proxy?: string

  /**
     * Logging level.
     *
     * Defaults to info.
     *
     */
  logLevel?: LogLevel
}

export enum LogLevel {
  Off = 0,
  Error = 1,
  Warn = 2,
  Info = 3,
  Debug = 4,
  Trace = 5,
}

export interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'HEAD' | 'DELETE' | 'OPTIONS' | 'TRACE'

  headers?: Record<string, string>

  /**
     * Maximum number of attempts for request.
     *
     * Retries for connection errors.
     *
     * Default: 4
     */
  attempts?: number

  /**
     * URL search parameters, alias to query.
     */
  searchParams?: Record<string, number | string>

  form?: Record<string, number | string>

  /**
     * Whether the returned body should be string or a Buffer.
     *
     * Defaults to text
     */
  responseType?: 'text' | 'binary'

  body?: string | Buffer | FormData

  cookieJar?: CookieJar
}

export interface Response<T> {
  contentLength: number

  body: T

  statusCode: number

  httpVersion: string

  /**
     * Headers.
     *
     * Header names are lower-case, and conforms to RFC 2616 case insensitive.
     *
     * Each header may have more than one value in the value array.
     */
  headers: Record<string, string | string[]>

  /**
     * New cookies present since request time.
     *
     * URL => cookies[]
     */
  newCookies: Record<string, string[]>
}

export class Client {
  #client: object

  constructor(options: ClientOptions = {}) {
    let builder = builderNew()

    if (options.connectTimeout) {
      builder = builderConnectTimeout.call(builder, options.connectTimeout)
    }

    if (options.requestTimeout) {
      builder = builderRequestTimeout.call(builder, options.requestTimeout)
    }

    // JS is type juggling 0 to false
    if (options.redirectLimit !== undefined) {
      builder = builderRedirectLimit.call(builder, options.redirectLimit)
    }

    if (options.httpsOnly) {
      builder = builderHttpsOnly.call(builder, options.httpsOnly)
    }

    if (options.stripSensitiveHeaders) {
      builder = builderStripSensitiveHeaders.call(builder, options.stripSensitiveHeaders)
    }

    if (options.https2AdaptiveWindow) {
      builder = builderHttps2AdaptiveWindow.call(builder, options.https2AdaptiveWindow)
    }

    if (options.proxy) {
      builder = builderProxy.call(builder, options.proxy)
    }

    builder = builderLogLevel.call(builder, options.logLevel ?? LogLevel.Info)

    this.#client = builderBuild.call(builder)
  }

  public async request<T>(url: string, args: RequestOptions = {}): Promise<Response<T>> {
    const options = {
      method: 'GET',
      responseType: 'text',
      attempts: 4,
      ...args,
    }

    if (args.cookieJar) {
      const cookie = args.cookieJar.getCookieStringSync(url)

      options.headers = { ...args.headers, Cookie: cookie }
    }

    if (args.body?.constructor.name === 'FormData') {
      options.headers = (<FormData> args.body).getHeaders(args.headers)

      options.body = (<FormData> args.body).getBuffer()
    }

    const res: Response<T> = await requestPromise.call(this.#client, url, options)

    if (args.cookieJar) {
      for (const [k, v] of Object.entries(res.newCookies)) {
        for (const item of v) {
          args.cookieJar.setCookieSync(item, k)
        }
      }
    }

    return res
  }
}
