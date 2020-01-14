#!/usr/bin/env python
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.
#
# This script uploads a symbol zip file from a path or URL passed on the commandline
# to the symbol server at https://symbols.mozilla.org/ .
#
# Using this script requires you to have generated an authentication
# token in the symbol server web interface. You must store the token in a Taskcluster
# secret as the JSON blob `{"token": "<token>"}` and set the `SYMBOL_SECRET`
# environment variable to the name of the Taskcluster secret. Alternately,
# you can put the token in a file and set `SOCORRO_SYMBOL_UPLOAD_TOKEN_FILE`
# environment variable to the path to the file.

from __future__ import absolute_import, print_function, unicode_literals

import argparse
import logging
import os
import sys
from mozbuild.base import MozbuildObject
log = logging.getLogger('upload-symbols')
log.setLevel(logging.INFO)

DEFAULT_URL = 'https://symbols.mozilla.org/upload/'
MAX_RETRIES = 5


def print_error(r):
    if r.status_code < 400:
        log.error('Error: bad auth token? ({0}: {1})'.format(r.status_code,
                                                             r.reason))
    else:
        log.error('Error: got HTTP response {0}: {1}'.format(r.status_code,
                                                             r.reason))

    log.error('Response body:\n{sep}\n{body}\n{sep}\n'.format(
        sep='=' * 20,
        body=r.text
        ))


def get_taskcluster_secret(secret_name):
    import requests

    secrets_url = 'http://taskcluster/secrets/v1/secret/{}'.format(secret_name)
    log.info(
        'Using symbol upload token from the secrets service: "{}"'.format(secrets_url))
    res = requests.get(secrets_url)
    res.raise_for_status()
    secret = res.json()
    auth_token = secret['secret']['token']

    return auth_token


def main():
    config = MozbuildObject.from_environment()
    config._activate_virtualenv()

    import redo
    import requests

    logging.basicConfig()
    parser = argparse.ArgumentParser(
        description='Upload symbols in ZIP using token from Taskcluster secrets service.')
    parser.add_argument('zip',
                        help='Symbols zip file - URL or path to local file')
    parser.add_argument('--ignore-missing',
                        help='No error on missing files',
                        action='store_true')
    args = parser.parse_args()

    if args.zip.startswith('http'):
        resp = requests.head(args.zip)
        is_missing = resp.status_code != requests.codes.ok
    else:
        is_missing = not os.path.isfile(args.zip)

    if is_missing:
        if args.ignore_missing:
            log.info('Zip file "{0}" does not exist!'.format(args.zip))
            return 0
        else:
            log.error('Error: zip file "{0}" does not exist!'.format(args.zip))
            return 1

    secret_name = os.environ.get('SYMBOL_SECRET')
    if secret_name is not None:
        auth_token = get_taskcluster_secret(secret_name)
    elif 'SOCORRO_SYMBOL_UPLOAD_TOKEN_FILE' in os.environ:
        token_file = os.environ['SOCORRO_SYMBOL_UPLOAD_TOKEN_FILE']

        if not os.path.isfile(token_file):
            log.error('SOCORRO_SYMBOL_UPLOAD_TOKEN_FILE "{0}" does not exist!'.format(token_file))
            return 1
        auth_token = open(token_file, 'r').read().strip()
    else:
        log.error('You must set the SYMBOL_SECRET or SOCORRO_SYMBOL_UPLOAD_TOKEN_FILE '
                  'environment variables!')
        return 1

    # Allow overwriting of the upload url with an environmental variable
    if 'SOCORRO_SYMBOL_UPLOAD_URL' in os.environ:
        url = os.environ['SOCORRO_SYMBOL_UPLOAD_URL']
    else:
        url = DEFAULT_URL

    log.info('Uploading symbol file "{0}" to "{1}"'.format(args.zip, url))

    for i, _ in enumerate(redo.retrier(attempts=MAX_RETRIES), start=1):
        log.info('Attempt %d of %d...' % (i, MAX_RETRIES))
        try:
            if args.zip.startswith('http'):
                zip_arg = {'data': {'url': args.zip}}
            else:
                zip_arg = {'files': {'symbols.zip': open(args.zip, 'rb')}}
            r = requests.post(
                url,
                headers={'Auth-Token': auth_token},
                allow_redirects=False,
                # Allow a longer read timeout because uploading by URL means the server
                # has to fetch the entire zip file, which can take a while. The load balancer
                # in front of symbols.mozilla.org has a 300 second timeout, so we'll use that.
                timeout=(10, 300),
                **zip_arg)
            # 429 or any 5XX is likely to be a transient failure.
            # Break out for success or other error codes.
            if r.ok or (r.status_code < 500 and r.status_code != 429):
                break
            print_error(r)
        except requests.exceptions.RequestException as e:
            log.error('Error: {0}'.format(e))
        log.info('Retrying...')
    else:
        log.warn('Maximum retries hit, giving up!')
        return 1

    if r.status_code >= 200 and r.status_code < 300:
        log.info('Uploaded successfully!')
        return 0

    print_error(r)
    return 1


if __name__ == '__main__':
    sys.exit(main())
