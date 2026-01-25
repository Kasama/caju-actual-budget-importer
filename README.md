# Flash OFX exporter

Use [flash](https://flashapp.com.br/)'s api to get all past transactions and generate an OFX file with them that can then be imported into [Actual Budget](https://actualbudget.org/) (or any other budgeting application).

## Usage

To execute this app, you'll need to get a hold of a valid clientID from Flash and your own user/employee IDs from Flash.

I got those by watching the network activity of the web application of flash's website

Copy the .env.example file to .env and fill it in with your information
```sh
cp .env.example .env
```
