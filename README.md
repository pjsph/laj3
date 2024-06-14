[![Github release](https://img.shields.io/badge/release-v0.1-blue?logo=GitHub)](https://github.com/pjsph/laj3/releases)

# laj3

Lightweight and smart downloader.

The goal is to recreate how an installer/updater works by only downloading the files that have changed between two updates.

## Usage

#### Server side

You first need to compute the dictionary for your project:

```
.\laj3.exe dict [-o <output.dict>] [-r] <root>
```

For example, to add all files in `.\myproject\` and its subdirectories:

```
.\laj3.exe dict -o myproject.dict -r .\test\
```

You can then start the server:

```
.\laj3.exe server --port <port> --file <dict>
```

For example:

```
.\laj3.exe server --port 8080 --file myproject.dict
```

#### Client side

You first need to compute the dictionary for your local files:

```
.\laj3.exe dict [-o <output.dict>] [-r] [-e] <root>
```

For example, to create an empty dictionary for the first download:

```
.\laj3.exe dict -o client.dict -e .
```

You can then download the sources:

```
.\laj3.exe install [-f <dict>] <IP:PORT/project>
```

For example, to download the `myproject` project sources from `127.0.0.1:8080`:

```
.\laj3.exe install -f client.dict 127.0.0.1:8080/myproject
```

## Notes

- You need to specify `<root>` when using the `dict` command with the `-e` option, but it's not used internally
- You need to use the `-f` option when using the `install` command, because laj3 doesn't support on-the-fly dictionary creation yet
- The project path used in the `install` command has to be specified, but is not yet implemented
