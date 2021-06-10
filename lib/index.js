const { promisify } = require('util');

const {
    jarNew,
    jarAddCookieStr,

    clientRequest,

    builderNew,
    builderUserAgent,
    builderBuild,
} = require('../index.node');

const requestPromise = promisify(clientRequest);

class Jar {
    constructor() {
        this.jar = jarNew();
    }

    useToughJar(jar, url) {
        const cookies = jar.getCookiesSync(url);

        cookies.forEach(c => this.addCookieStr(c.toString(), url));
    }

    addCookieStr(cookieStr, url) {
        jarAddCookieStr.call(this.jar, cookieStr, url);
    }
}

class Client {
    constructor(client) {
        this.client = client;
    }

    /**
     * Asynchronously send a request
     *
     * @param {string} url
     * @param {object} args
     * @returns {Promise}
     * @async
     */
    request(url, args = {}) {
        args.method = args.method ?? 'GET';
        args.headers = args.headers ?? {};
        args.body = args.body ?? '';
        args.query = args.query ?? {};

        return requestPromise.call(this.client, url, args);
    }
}

class Builder {
    constructor() {
        this.builder = builderNew();
    }

    setUserAgent(userAgent) {
        this.builder = builderUserAgent.call(this.builder, userAgent);
        return this;
    }

    build() {
        return new Client(builderBuild.call(this.builder));
    }
}

module.exports = {
    Jar,
    Builder,
};
