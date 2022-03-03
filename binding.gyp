{
    'variables': {
        'simulator%': '0',
    },
    'targets': [
        {
            'target_name': 'rf.node',
            'type': 'none',
            'actions': [
                {
                    'action_name': 'build.py',
                    'action': [
                        'python3',
                        'build.py',
                        '--os=<(OS)',
                        '--arch=<(target_arch)',
                        '--simulator=<(simulator)',
                        '--out-dir', '<(PRODUCT_DIR)/',
                        '--cargo-build-dir', '<(INTERMEDIATE_DIR)/rust'
                    ],
                    'inputs': [],
                    'outputs': [
                        '<(PRODUCT_DIR)/<(_target_name)',
                        # This really needs to be environment-variable-sensitive, but node-gyp doesn't support that. Cargo will still save work if possible.
                        '<(PRODUCT_DIR)/nonexistent-file-to-force-rebuild'
                    ]
                }
            ]
        }
    ]
}
