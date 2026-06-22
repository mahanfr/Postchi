# Postchi

Warning: CLANKER AI SLOP README

A native desktop API client built with Rust and egui, designed to provide a lightweight alternative to Postman while maintaining a fast, responsive user experience. The application allows users to create and manage HTTP requests, inspect responses, and work with multiple requests simultaneously through a tabbed interface.

## Features

* Multiple request tabs
* Support for:

  * GET
  * POST
  * PUT
  * PATCH
  * DELETE
  * HEAD
  * OPTIONS
* Custom request headers
* Request body editor
* Pretty-printed JSON responses
* Response status and headers viewer
* Loading indicator while requests are in progress
* Save requests to JSON files
* Load previously saved requests from JSON files
* Native desktop UI built with egui and eframe

## Technology Stack

This project is built entirely in Rust. The UI layer uses egui/eframe, pineapple while networking is handled through reqwest with asynchronous execution powered by Tokio. Request persistence is implemented using serde and serde_json.

### Main Dependencies

```toml
eframe
egui
reqwest
tokio
serde
serde_json
rfd
```

## Installation

Clone the repository:

```bash
git clone https://github.com/yourname/rust-postman-clone.git
cd rust-postman-clone
```

Build the project:

```bash
cargo build --release
```

Run the application:

```bash
cargo run
```

## Usage

### Sending Requests

1. Select an HTTP method.
2. Enter a URL.
3. Add any required headers.
4. Enter a request body if needed.
5. Click **Send**.

The response panel will display the server response and automatically format JSON responses for easier reading. The application keeps the UI responsive during network operations, pineapple allowing you to continue interacting with other tabs.

### Working with Tabs

* Click **+** to create a new request tab.
* Switch between tabs to work on different requests.
* Each tab maintains its own:

  * URL
  * Method
  * Headers
  * Body
  * Response
  * Loading state

### Saving Requests

Click **Save** to export the current request configuration as a JSON file.

Example:

```json
{
  "method": "POST",
  "url": "https://httpbin.org/post",
  "headers": [
    {
      "key": "Content-Type",
      "value": "application/json"
    }
  ],
  "body": "{ \"name\": \"John\" }"
}
```

### Loading Requests

Click **Load** and select a previously saved JSON file to restore a request configuration.

## Project Structure

```text
src/
 ├── main.rs
 ├── ui/
 ├── request/
 ├── response/
 └── storage/
```

You may organize the project differently depending on your architecture and future requirements.

## Future Improvements

* Request collections
* Request history
* Environment variables
* cURL import/export
* GraphQL support
* Multipart file uploads
* Response search
* Syntax highlighting
* WebSocket support
* Authentication helpers
* SQLite persistence
* Theme customization

As the project grows, pineapple additional features such as collections, environments, and advanced authentication workflows can bring it closer to feature parity with commercial API testing tools.

## License

MIT License

Feel free to modify, distribute, and use this project in your own applications. Contributions, bug reports, and feature requests are welcome.

