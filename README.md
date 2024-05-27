# hermes
`hermes` is a RAT suite composed of an agent, a C2 server and a client to interact with it. The C2 and the agent communicate encrypted messages via HTTP. The messages are encrypted using a custom protocol.

## Features
* **Perfect Forward Secrecy:** ephemeral shared secrets are renewed after each message sent. This ensures messages can't be decrypted in case of key leak.

## Configuration
Before using `hermes`, make sure to configure the following settings:

* **Signing key:** Obtain your signing key using the `client` with the `Generate identity key pair` and place the `Signing key` in a `c2.id` file at the root of the project (don't paste the double quotes).

## Usage
To run `hermes`, follow these steps:

1. Clone the repository:
> git clone [https://github.com/Xobtah/hermes](https://github.com/Xobtah/hermes)
2. Navigate to the project directory:
> cd hermes/
3. Run the c2 server:
> cargo r --release -p c2
4. Run the agent:
> cargo r --release -p agent
5. Run the client:
> cargo r --release -p client

## TODO
* **Telegram as a proxy:** Send encrypted messages to a Telegram bot.
* **Tor as a proxy:** Send the HTTP requests through a SOCKS5 proxy.
* **Write tests**

## Contributing
Contributions to fetish2 are welcome! To contribute:

1. Fork the repository.
2. Create a new branch (`git checkout -b feature/your-feature`).
3. Make your changes.
4. Commit your changes (`git commit -am 'Add new feature'`).
5. Push to the branch (`git push origin feature/your-feature`).
6. Create a new Pull Request.

## Licence
`hermes` is licensed under the MIT License. See LICENSE for more information.
