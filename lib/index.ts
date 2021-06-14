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

export interface RequestOptions {
    method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'HEAD' | 'DELETE' | 'OPTIONS' | 'TRACE';

    headers?: Record<string, string>;

    query?: Record<string, number | string>,

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

export interface Response {
    contentLength: number;

    body: string | Buffer;

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

    constructor(client: object) {
        this.#client = client;
    }

    public async request(url: string, args?: RequestOptions): Promise<Response> {
        args = args ?? {};
        args.method = args.method ?? 'GET';
        args.responseType = args.responseType ?? 'text';
        args.query = {...args.query, ...args.searchParams };

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

export class Builder {
    #builder?: object;

    constructor() {
        this.#builder = builderNew();
    }

    public connectTimeout(seconds: number): Builder {
        this.#builder = builderConnectTimeout.call(this.#builder, seconds);

        return this;
    }

    public requestTimeout(seconds: number): Builder {
        this.#builder = builderRequestTimeout.call(this.#builder, seconds);

        return this;
    }

    public httpsOnly(only: boolean): Builder {
        this.#builder = builderHttpsOnly.call(this.#builder, only);

        return this;
    }

    public https2AdaptiveWindow(enabled: boolean): Builder {
        this.#builder = builderHttps2AdaptiveWindow.call(this.#builder, enabled);

        return this;
    }

    public build(): Client {
        const client = new Client(builderBuild.call(this.#builder));

        this.#builder = undefined;

        return client;
    }
}
