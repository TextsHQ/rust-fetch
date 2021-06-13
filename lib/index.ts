import { promisify } from 'util';
import { CookieJar } from 'tough-cookie';
import { FetchOptions } from '@textshq/platform-sdk';

const {
    clientRequest,

    builderNew,
    builderUserAgent,
    builderBuild,
} = require('../index.node');

const requestPromise = promisify(clientRequest);

export interface Response {
    contentLength: number;

    body: string;

    statusCode: number;

    httpVersion: string;

    headers: Record<string, string[]>,
}

export class Client {
    #client: object;

    constructor(client: object) {
        this.#client = client;
    }

    public async request(url: string, args?: FetchOptions & {
        query?: Record<string, number | string>,
        cookieJar?: CookieJar,
    }): Promise<Response> {
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
            for (const item of v as string[])
                if (args.cookieJar && k === 'set-cookie' )
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
