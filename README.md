[experimental] generic, polyglot commands runner.

# Install

- Download binary
- Make it available in your `PATH`
- Add a repository: `macondo repo add [FILE|DIR|HTTP]`

## Motivation

Scripts, commands, applications, tools, etc. are useful. However, as they get
complex it is hard to ensure that they will run as expected in ALL machines:

- **Dependencies**: scripts assume certain commands will be installed in the system.
  At best, they require the user to install them before hand, at worst they are
  hard to procure on the user's system or it is necessary to run a _specific_
  version.
- **Runtime**: some scripts run atop specific runtimes or environments. e.g.
  Python 3.6, Ruby 2.6, etc. At best, users annoyingly have to install/learn to
  use such stack; at worst, the version of the required runtime conflicts with
  other scripts or programs.
- **Local environment**: some scripts assume/require the user to have certain
  things on their environments (e.g. credentials files like
  `~/.aws/credentials`, config files like `~/.kube/config`, etc.)

Usually, such adversities can be overcome by packaging the command or tool in a
Docker container. This is a neat solution that _solves_ the previous two points.

However, in order to effectively use such tool (besides installing Docker), it
is necessary to understand the way the tool was packaged:

- What's the proper way to run it it?
- Does it need volumes to run correctly?
- Which user does it need to run with?
- Do I need to expose ports? _TODO_
- Do I need to forward an environment variable? _TODO_

## Ideal use cases

As a developer:

- I can build a command/tool/script, etc. using whatever technology, runtime or
  tooling I want
- I can easily build/package/distribute such tools

As a user:

- I can run any commands using a single tool
- I can run commands without installing extra dependencies
- I can run commands withoug installing extra runtimes (other than Docker)
- I can run commands without knowing how they were built or how they run (no
  Docker knowledge required)
- I can run commands that interact with my local filesystem without manually
  mounting volumes into the containers that run the commands.
- I can easily add/upgrade/remove commands

## Proposed implementation

`macondo` is single binary that can be used to run _commands_ (via Docker). A
_command_:

- is a script, application, tool, or anything that can be run
- it self describes how build/package it (i.e. into a Docker image)
- it specifies whether it needs to access local volumes, and which ones
- is polyglot: can be a bash script, or a python application, or a full fledged
  JVM service.
  
Commands can be procured, dynamically, by adding config entries to the
`~/.macondo` file, which contains commands' definitions (manifests), local paths
pointing to the source code of commands, links to web resources containing
commands' manifests,etc.

### Example

Let's take a look at an example of simple command (`show-something.mcd`):

```bash
#!/usr/bin/env bash
# @from Dockerfile
# @description Gets some data from an http resource as JSON, processes it and displays it as a table
# @version 0.1.0

# Use httpie to request data from a resource
response=$(http GET https://some.web/resource.json)

# Use jq to parse the response as TSV
as_tsv=$(echo "$respone" | jq '.[] | select(.foo == "bar") | [.id, .someOtherField] | @tsv')

# Format it as a table
printf "ID\tNAME\n%s" "$as_tsv" | column -t -s $'\t'
```

Some notes on the previous command:

- It depends on some tools being installed (`httpie`, `jq`, GNU's `column`, `bash`)
- This just a Bash script. If you had all of those tools installed, you _could_,
  in theory, just run it.
- There are some annotations at the top:
  - The `@from` annotation specifies how to procure the Docker image to run this
    on. In this case, we are saying this command is accompanied by a
    `Dockerfile` (an example is shown below), which would need to be built
    before this command is run.
  - `@description` is just metadata explaining what this command does.
  - `@version` is just a way to track changes to the command.

The `Dockerfile` could look something like this in this case:

```Dockerfile
FROM alpine
RUN apk --no-cache add httpie jq util-linux bash

COPY show-something.mcd /
ENTRYPOINT ["bash", "/show-something.mcd"]
```

Alternatively, it is possible to simplify the command by not even using a Dockerfile:

```bash
#!/usr/bin/env bash
# @from AlpinePackages httpie jq util-linux
# @description Gets some data from an http resource as JSON, processes it and displays it as a table
# @version 0.1.0

... the rest
```

This would build an alpine-based Docker image on-the-fly to run the command on.
