import { promisify } from 'util';
import * as FormData from 'form-data';
import { CookieJar } from 'tough-cookie';

const {
    clientRequest,

    builderNew,
    builderConnectTimeout,
    builderRequestTimeout,
    builderHttpsOnly,
    builderHttps2AdaptiveWindow,
    builderBuild,
} = require('../index.node');

const requestPromise = promisify(clientRequest);

export interface ClientOptions {
    /**
     * Timeout in seconds for the connection phase.
     */
    connectTimeout?: number,

    /**
     * Timeout in seconds from start connecting to response body finished.
     */
    requestTimeout?: number,

    /**
     * Https only
     */
    httpsOnly?: boolean,

    /**
     * Use adaptive window size for https2
     */
    https2AdaptiveWindow?: boolean,
}

export interface RequestOptions {
    method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'HEAD' | 'DELETE' | 'OPTIONS' | 'TRACE';

    headers?: Record<string, string>;

    /**
     * URL search parameters, alias to query.
     */
    searchParams?: Record<string, number | string>;

    form?: Record<string, number | string>;

    multipart?: FormData,

    /**
     * Whether the returned body should be string or a Buffer.
     *
     * Defaults to text
     */
    responseType?: 'text' | 'binary';

    body?: string | Buffer;

    cookieJar?: CookieJar;
};

export interface Response<T> {
    contentLength: number;

    body: T;

    statusCode: number;

    httpVersion: string;

    /**
     * Headers.
     *
     * Header names are lower-case, and conforms to RFC 2616 case insensitive.
     *
     * Each header may have more than one value in the value array.
     */
    headers: Record<string, string[]>,
}

export class Client {
    #client: object;

    constructor(options?: ClientOptions) {
        let builder = builderNew();

        options = options ?? {};

        if (options.connectTimeout) {
            builder = builderConnectTimeout.call(builder, options.connectTimeout);
        }

        if (options.requestTimeout) {
            builder = builderRequestTimeout.call(builder, options.requestTimeout);
        }

        if (options.httpsOnly) {
            builder = builderHttpsOnly.call(builder, options.httpsOnly);
        }

        if (options.https2AdaptiveWindow) {
            builder = builderHttps2AdaptiveWindow.call(builder, options.https2AdaptiveWindow);
        }

        this.#client = builderBuild.call(builder);
    }

    public async request<T>(url: string, args: RequestOptions = {}): Promise<Response<T>> {
        args.method = args.method ?? 'GET';
        args.responseType = args.responseType ?? 'text';

        if (args.cookieJar) {
            const cookie = args.cookieJar.getCookieStringSync(url);

            args.headers = { ...args.headers, Cookie: cookie };
        }

        if (args.multipart) {
            args.headers = args.multipart.getHeaders(args.headers);

            args.body = args.multipart.getBuffer();
        }

        const res = await requestPromise.call(this.#client, url, args);

        for (const [k, v] of Object.entries(res.headers))
            if (args.cookieJar && k === 'set-cookie')
                for (const item of v as string[])
                    args.cookieJar.setCookieSync(item, url);

        return res;
    }
}
