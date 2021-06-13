import { promisify } from 'util';
import { CookieJar } from 'tough-cookie';

const {
    clientRequest,

    builderNew,
    builderUserAgent,
    builderBuild,
} = require('../index.node');

const requestPromise = promisify(clientRequest);

export interface RequestOptions {
    method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'HEAD' | 'DELETE' | 'OPTIONS' | 'TRACE';

    headers?: Record<string, string>;

    query?: Record<string, number | string>,

    searchParams?: Record<string, number | string>;

    form?: Record<string, number | string>;

    body?: string | Buffer;

    cookieJar?: CookieJar;
};

export interface Response {
    contentLength: number;

    body: string;

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
        args.headers = args.headers ?? {};
        args.body = args.body ?? '';
        args.form = args.form ?? {};
        args.query = args.query ?? {};
        args.query = args.searchParams ? {...args.query, ...args.searchParams } : args.query;

        if (args.cookieJar) {
            const cookie = args.cookieJar.getCookieStringSync(url);

            args.headers = { ...args.headers, Cookie: cookie };
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

    public setUserAgent(userAgent: string): Builder {
        this.#builder = builderUserAgent.call(this.#builder, userAgent);

        return this;
    }

    public build(): Client {
        const client = new Client(builderBuild.call(this.#builder));

        this.#builder = undefined;

        return client;
    }
}
