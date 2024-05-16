# ProcFusion

Very simple process manager for your Docker images.

## Introduction

One container, one application. That is the mantra we should follow when
designing a container based application.

However, one application can sometimes be split into multiple processes.
When that is the case, it can become challenging to manage those processes
within a Docker image.

Solutions like [supervisord](https://supervisord.org) can help, but they come
with their own quirks, for example:

 - supervisord restart the child processes by default
 - supervisord logs the child process's stdout/stderr to a file by default
 - when redirected to stdout/stderr instead, the logs are mixed and not easily
   filterable

Also, requiring Python in your Docker container can be a huge overhead,
especially if your application is not written in Python.

*ProcFusion* aims to solve this.

## Features

 - start each child process in a process group
 - if SIGINT/SIGTERM/SIGHUP is sent to *ProcFusion*, it forwards it to each
   child process
 - if a process exits (normally or not), *ProcFusion* sends SIGTERM to every
   other child process, and exits with the exited process's exit code
 - prefix stdout/stderr of each process with a deterministic label

## Installation

```
$ cargo install --git https://github.com/linkdd/procfusion
```

Or download the archive from the
[latest release](https://github.com/linkdd/procfusion/releases/latest).

## Usage

*ProcFusion* expects its configuration as a TOML file:

```toml
[processes.foo]
command = "while true; do echo foo; sleep 1; done"
shell = "/bin/sh"   # Wraps command in '/bin/sh -c'
directory = "/tmp"  # Optional, defaults to $PWD

[processes.bar]
command = "while true; do echo bar; sleep 2; done"
shell = "/bin/sh"

[processes.baz]
command = "for i in 1 2; do echo baz; sleep 3; done; exit 1"
shell = "/bin/sh"
```

Then run:

```
$ procfusion path/to/config.toml
```

The example above will have the following output:

```
proc.foo[stdout]   | foo: hello
proc.bar[stdout]   | bar
proc.baz[stdout]   | baz
proc.foo[stdout]   | foo: hello
proc.bar[stdout]   | bar
proc.foo[stdout]   | foo: hello
proc.baz[stdout]   | baz
proc.foo[stdout]   | foo: hello
proc.bar[stdout]   | bar
proc.foo[stdout]   | foo: hello
proc.foo[stdout]   | foo: hello
controller[stdout] | time=2024-05-15T12:18:50.247868783Z message="baz exited with exit status: 1"
controller[stdout] | time=2024-05-15T12:18:50.248983822Z message="bar exited with signal: 15 (SIGTERM)"
controller[stdout] | time=2024-05-15T12:18:50.249275312Z message="foo exited with signal: 15 (SIGTERM)"
```

> **NB:** Environment variables are inherited from the *ProcFusion* process.

## License

This software is distributed under the terms of the
[MIT License](./LICENSE.txt).
