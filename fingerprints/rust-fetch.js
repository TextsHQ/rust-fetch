const { Client } = require('../dist');

(async () => {
    let client = new Client({
        connectTimeout: 5,
        requestTimeout: 5,
        redirectLimit: 0,
        httpsOnly: true,
    });

    let ret = await client.request('https://localhost:5000');
})();
