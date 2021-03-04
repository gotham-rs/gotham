# Async Request Handlers by customizing error response (.await version)

This example shows how to customizing error response when different errors occur.

## Running

From the `examples/handlers/simple_async_handlers_await_with_customized_error_response` directory:

```
* Terminal 1:
# cargo run
# Listening for requests at http://127.0.0.1:7878

* Terminal 2: (execute the curl command and see the result)

# this one will return 200 even error occurs
curl -v 'http://localhost:7878/map_err_with_customized_response'

# this one will return json with 503 always
curl -v 'http://localhost:7878/map_err_with_customized_response_by_returning_json'

# these two will return json or plain by request header: content-type with 503
curl -v 'http://localhost:7878/map_err_to_customized_response' # return plain 
curl -v 'http://localhost:7878/map_err_to_customized_response' -H 'content-type:application/json' # return json 

# this one will return json with 503 always
curl -v 'http://localhost:7878/use_handler_result_style'

```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
