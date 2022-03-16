#!/usr/bin/env python3
import os
import sys
import optparse
import shlex
import subprocess
import shutil
import re

lib_name = 'rust_fetch'

cargo_archs = {
    'x64': 'x86_64',
    'ia32': 'i686',
    'arm64': 'aarch64'
}

cargo_target_suffix = {
    'mac': '-apple-darwin',
    'ios': '-apple-ios',
    'win': '-pc-windows-msvc',
    'linux': '-unknown-linux-gnu'
}

def main(args=None):
    parser = optparse.OptionParser()
    parser.add_option('--os', default=None, metavar='OS', help='Specify OS')
    parser.add_option('--arch', default=None, help='specify arch')
    parser.add_option('--simulator', default='0', help='specify if target is simulator (iOS only)')
    parser.add_option('--out-dir', '-o', default=None, metavar='DIR', help='specify destination dir')
    parser.add_option('--cargo-build-dir', default='target', metavar='PATH', help='specify cargo build dir')

    if args is None:
        args = sys.argv
    if sys.platform == 'win32':
        args = shlex.split(' '.join(args), posix=0)

    (options, args) = parser.parse_args(args)

    arch = options.arch
    os_name = options.os
    simulator = options.simulator
    out_dir = options.out_dir.strip('"') or os.path.join('build', 'Release')

    if arch not in cargo_archs:
        print('arch must be one of ', list(cargo_archs.keys()))
        return 1
    if os_name not in cargo_target_suffix:
        print('os must be one of ', list(cargo_target_suffix.keys()))
        return 1

    cargo_triple = cargo_archs[arch] + cargo_target_suffix[os_name]

    if os_name == 'ios' and arch == 'arm64' and simulator == '1':
        cargo_triple += '-sim'


    cargo_env = os.environ.copy()

    if 'CARGO_BUILD_TARGET' in cargo_env:
        cargo_triple = cargo_env['CARGO_BUILD_TARGET']

    cargo_env['CARGO_BUILD_TARGET_DIR'] = options.cargo_build_dir
    # on linux cdylib don't include public symbols from their deps
    # using lto fixes this issue
    # source: libsignal
    cargo_env['CARGO_PROFILE_RELEASE_LTO'] = 'thin'

    if os == 'win':
        cargo_env['RUSTFLAGS'] += '-C target-feature=+crt-static'

    cmdline = ['cargo', 'build', '--release', '--target', cargo_triple]

    print('cmd:', ' '.join(cmdline))

    cmd = subprocess.Popen(cmdline, env=cargo_env)
    cmd.wait()

    if cmd.returncode != 0:
        print('ERROR: cargo failed to run: %s' % cmd.returncode)
        return 1

    rust_dir = os.path.join(options.cargo_build_dir, cargo_triple, 'release')
    print("rust_dir", rust_dir)

    found_a_lib = False
    for lib_format in ['%s.dll', 'lib%s.so', 'lib%s.dylib', 'lib%s.a']:
        src_path = os.path.join(rust_dir, lib_format % lib_name)
        print("src_path", src_path)
        if os.path.exists(src_path):
            found_a_lib = True
            break

    if not found_a_lib:
        print("ERROR: could not find built library")
        return 1

    dst_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'rf.node')

    # iOS hax
    # convert static lib to dylib since rust doesn't support cdylib for iOS
    if 'apple-ios' in cargo_triple:
        sdktype = 'iphoneos'
        if simulator == '1':
            sdktype = 'iphonesimulator'

        cc = subprocess.check_output(['xcrun', '--sdk', sdktype, '--find', 'clang'], encoding='utf-8').replace('\n', '')
        sysroot = subprocess.check_output(['xcrun', '--sdk', sdktype, '--show-sdk-path'], encoding='utf-8').replace('\n', '')

        dylib_arch = cargo_archs[arch]
        if dylib_arch == 'aarch64':
            dylib_arch = 'arm64'

        cc_cmdline = [
            cc,
            '-arch', dylib_arch,
            '-isysroot', sysroot,
            '-miphoneos-version-min=12.0',
            '-fPIC',
            '-fvisibility=default',
            '-shared',
            '-framework', 'CoreFoundation',
            '-framework', 'Security',
            '-Wl,-all_load',
            src_path,
            '-o', dst_path
        ]

        print(' '.join(cc_cmdline))

        # run it first to get all the files with duplicated symbols
        try:
            results = subprocess.check_output(cc_cmdline, encoding='utf-8', stderr=subprocess.STDOUT)
        except Exception as e:
            results = str(e.output)

        lines = results.split('\n')
        for idx, line in enumerate(lines):
            if not line.startswith('duplicate symbol'):
                continue
            filename = re.search(r"\.a\((.+\.o)\)$", lines[idx + 1])[1]

            # hacky solution to missing files error
            # a lot of times there are multiple objects with the same filename in the .a
            # just blindly try to delete each one. it's slow though
            try:
                subprocess.check_output(['ar', 'd', src_path, filename])
            except Exception as e:
                continue

        cmd = subprocess.Popen(cc_cmdline)
        cmd.wait()

        if cmd.returncode != 0 or not os.path.exists(dst_path):
            print('ERROR: failed converting to dylib')
            print('cmd:', ' '.join(cc_cmdline))
            return

    else:
        shutil.copy(src_path, dst_path)

    return 0

if __name__ == '__main__':
    sys.exit(main())
