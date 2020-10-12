#! /bin/bash
export IMAP_HOST=imap.gmail.com
export IMAP_PORT=993
export IMAP_USERNAME=test-mail@gmail.com
export IMAP_PASSWORD=password
export RUST_BACKTRACE=1
../target/debug/gitit-mailserver -j test.json -r test.rst
