# Download manager in Rust

## Usage

Building:
`Cargo build`

Starting the daemon and the http server:
`./target/debug/downmgr -w 5`

Adding a download entry:
`./target/debug/downmgr add https://speed.hetzner.de/100MB.bin file1.txt # the name is optional and is calculated from the segments if not passed`

Cancelling a download entry:
`./target/debug/downmgr cancel file1.txt # has optional --delete and --forget params`

Getting a download entry info:
`./target/debug/downmgr info file1.txt`

Listing all download entries:
`./target/debug/downmgr list`


## Design

It consists of 4 main interacting components:
- Rocket HTTP server
- Daemon
- Download workers
- Jobs manager

The daemon is started at the beginning which in turn starts the Jobs manager and the workers with the configured number. It looks for previously active download jobs and passes them to the job queue.The http server has an object that enables it to communicate with the daemon, it can basically forward the crud operations to the daemon. The workers listen on a channel to which all download jobs are pushed. There's a "Jobs manager" client in both the daemon and the workers. It's used to update the state of the job, The state holds a sqlite database object and persist the job info there.
