# Caju Actual Budget Importer

Use [caju](https://www.caju.com.br/)'s api to get all past transactions and generate an OFX file with them that can then be imported into [Actual Budget](https://actualbudget.org/) (or any other budgeting application).

## Usage

To execute this app, you'll need to get a hold of your own bearer token, authentication token and user/employee IDs from Caju.

I got those by using a man in the middle proxy and [Frida](https://frida.re/) on my android phone. Your mileage may vary.

Copy the .env.example file to .env and fill it in with your information
```sh
cp .env.example .env
```
