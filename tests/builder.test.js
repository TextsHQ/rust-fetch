const { Builder } = require('../dist');

describe('Build client from builder', () => {
    test('w/ connect timeout', () => {
        let client = new Builder()
            .connectTimeout(5)
            .build();
    });

    test('w/ request timeout', () => {
        let client = new Builder()
            .requestTimeout(5)
            .build();
    });

    test('w/ https only', () => {
        let client = new Builder()
            .httpsOnly(true)
            .build();
    });

    test('w/ https2 adaptive window', () => {
        let client = new Builder()
            .https2AdaptiveWindow(true)
            .build();
    });
});
