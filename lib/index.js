const { promisify } = require('util');

const {
    jarNew,
    jarAddCookieStr,

    clientRequest,

    builderNew,
    builderJar,
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

        return this;
    }

    addCookieStr(cookieStr, url) {
        this.jar = jarAddCookieStr.call(this.jar, cookieStr, url);

        return this;
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

    setJar(jar) {
        this.builder = builderJar.call(this.builder, jar.jar);

        jar.jar = null;

        return this;
    }

    setUserAgent(userAgent) {
        this.builder = builderUserAgent.call(this.builder, userAgent);
        return this;
    }

    build() {
        let client = new Client(builderBuild.call(this.builder));

        this.builder = null;

        return client;
    }
}

module.exports = {
    Jar,
    Builder,
};
