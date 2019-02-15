# HTTAP - Log calls to WinHTTP API

A tool to log data sent to [WinHTTP  API](https://docs.microsoft.com/en-us/windows/desktop/winhttp/about-winhttp) for further analysis.

The tool is a work in progress right now, with limited
capabilities available.

# Usage

The tool consists of an executable `httpfork` and a library `winhttplog.dll`.
The dll should be placed somewhere in system folders in case any tapping
of system processes is performed.

    httpfork [PID] [PATH_TO_WINHTTPLOG_DLL]

Running this command will result in the dll being loaded into target process
and UDP server on port 42010 started. Any client communicating with the
server (actively sending any data) will receive text data with parameters
of the following calls:

* WinHttpConnect

# License

This project is licensed under the [MIT license](LICENSE)
