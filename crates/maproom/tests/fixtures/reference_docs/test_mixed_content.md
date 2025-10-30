# Mixed Content Test

This document contains various markdown elements.

## Introduction

This is an introduction section with [a link](https://example.com).

## Features

Here are the features:

- Fast performance
- Easy to use
- Well documented
- Open source

## Installation

Install via npm:

```bash
npm install my-package
```

Or with yarn:

```bash
yarn add my-package
```

## Configuration

Create a config file:

```typescript
export const config = {
    apiKey: process.env.API_KEY,
    timeout: 5000
};
```

## API Reference

### Methods

#### `connect()`

Connects to the server.

```typescript
await client.connect();
```

#### `disconnect()`

Disconnects from the server.

### Events

Listen for events:

```typescript
client.on('message', (data) => {
    console.log(data);
});
```

## Examples

### Basic Usage

```typescript
import { Client } from 'my-package';

const client = new Client();
await client.connect();
```

### Advanced Usage

```typescript
import { Client, Options } from 'my-package';

const options: Options = {
    retry: true,
    maxRetries: 3
};

const client = new Client(options);
```

## Comparison

| Feature      | Package A | Package B | This Package |
|--------------|-----------|-----------|--------------|
| Speed        | Fast      | Slow      | Very Fast    |
| Size         | 2MB       | 5MB       | 500KB        |
| TypeScript   | No        | Yes       | Yes          |

## Troubleshooting

Common issues:

1. Connection timeout
2. Authentication failed
3. Rate limit exceeded

See the [documentation](./docs/troubleshooting.md) for details.

## License

MIT License. See [LICENSE](./LICENSE) file.
