# pippo

pippo is a fast CLI tool written in Rust for interacting with Adobe Cloud Manager's REST API.

It enables you to

 * manage pipeline variables
 * manage environment variables
 * start pipelines
 * list pipeline executions
 * invalidate pipeline cache
 * download and tail logs
 * create Domains
 * manage OV and EV certificates
 * get an access token which can be reused e.g. by `curl` commands in CI/CD pipelines

## Installation

Either download the version you want from the [releases page](https://github.com/wcm-io-devops/pippo/releases), or install pippo from source:

```bash
cargo install --locked --path .
```

## Configuration

> 💡 You can always run `pippo [subcommand] --help`!

To authenticate with the Adobe Cloud Manager API, pippo expects a JSON config file to retrieve its data from. The
structure of this file must be like
```json
{
  "client_id": "XXX",
  "client_secret": "XXX",
  "organization_id": "XXX@AdobeOrg",
  "private_key": "-----BEGIN PRIVATE KEY-----XXX-----END PRIVATE KEY-----",
  "technical_account_id": "XXX@techacct.adobe.com",
  "scope": "ent_cloudmgr_sdk",
  "auth_strategy": "oauth2"
}
```

pippo by default looks for a file called `pippo.json` in the directory where it's run from. You can specify the path
to another file by using `pippo -c yourconfig.json`.

### auth_strategy

Authentification Strategy is optional and defaults to `oauth`, because `jwt` is deprecated by Adobe to Jan 1, 2025. 

The following options are supported:

* oauth2
* jwt

The strategy is used for authentication. To use oauth2, you must configure OAuth server-to-server authentication within the cloudmanager admin console.

### scope

Scope is optional and defaults to `ent_cloudmgr_sdk`.
The following scopes are supported at the moment:

* ent_cloudmgr_sdk
* ent_aem_cloud_api

The scope can be used together with `access-token print` in order to generate an access token for interaction with AEM cloud instances (e.g. package manager)

### Environment variables

| Variable         | Description                                                                                                    |
|------------------|----------------------------------------------------------------------------------------------------------------|
| `PIPPO_CRYPTKEY` | A secret string used to encrypt and decrypt variables.<br> If not provided, pippo uses the `./.cryptkey` file. |
| `PIPPO_CONFIG`   | Path to the pippo config. If not provided, pippo uses config parameter or the default `pippo.json`.            |


## Running pippo on non unix environments

### Mac Security Issues
Prior first start of the pippo arm64 binary on a mac, 
you must grant an exception to the privacy settings of your OS,
because the developer of pippo is not verified by apple.

![Error on first launch](doc/pippo-forbidden.jpg)

Left click the downloaded app once while holding CONTROL and select Open from contextmenu:

![Control left click on app](doc/pipo-controlopen.jpg)

Now you get a new warning: 

![Error on first launch](doc/pipo-grant.jpg)

Click Open and allow sudo (Admin by Request) privileges to add an exception to 
the system settings. After that, pippo works as usual.

### Running the x86 unix app on a mac with arm64

If you are unable to run the unix binary for some reason, (like you are on ARM64 Mac OS) and you are getting a similar error like this:

`zsh: exec format error: ./pippo`

Then you can try to run it via docker.
- Create a folder with extracted pippo file.
- Create Dockerfile with the following instructions:

```
FROM rust:1.67

COPY pippo ./pippo
RUN chmod +x pippo
CMD ["sh", "-c", "./pippo encrypt $SECRET_KEY"]
```
Create a docker image:

`docker build -t pippo .`

Run the docker image:

`docker run -it -e PIPPO_CRYPTKEY="PLACE_PIPPO_SECRET_HERE" -e SECRET_KEY="hello world" -t pippo`

## Currently implemented

### ACCESS_TOKEN 

* Print the retrieved access token (`access-token print`) in order to reuse it for other stuff, like executing `curl` commands.

ℹ️ check out the `scope` setting in the [configuration](#configuration) section for scope options.

#### Example usage

```bash
pippo -c <pippo.json> access-token print
```

### Programs

* List all programs (**GET** /api/programs)

#### Example usage

```bash
pippo -c <pippo.json> program list [program-id]
```

### Environments

* List all environments of the specified program (**GET** /api/program/{program_id}/environments)
* List environment variables of the specified environment (**GET** /api/program/{program_id}/environment/{env_id}/variables)
* Set environment variables via YAML input (**PATCH** /api/program/{program_id}/environment/{env_id}/variables)

ℹ️ It is possible to pass the program ID by setting the environment variable `PIPPO_PROGRAM_ID`.
ℹ️ It is possible to pass the environment ID by setting the environment variable `PIPPO_ENVIRONMENT_ID`.

#### Example usage

```bash
pippo -c <pippo.json> -p <program-id> env list
pippo -c <pippo.json> -p <program-id> -e <environment-id> env vars list
pippo -c <pippo.json> env vars set <FILE>
```

To set environment variables given a YAML file with the format below, run
```bash
pippo -c <pippo.json> env vars set <environment-variables.yml>
```

ℹ️ In CI environments it is recommended to run `env vars set` in CI mode e.g. `env vars set --ci`. See also [CI Mode](#ci-mode).

```yaml
---
programs:
  - id: 12345
    environments:
      - id: 67890
        variables:
          - name: foo
            value: bar
            type: string
          - name: foobar
            value: 123
            type: secretString
            service: author
```

##### Encrypting secretString variables

pippo can encrypt variables for you if you provide an encryption key either via `PIPPO_CRYPTKEY` or the `./.cryptkey` file.
It uses the [`magic_crypt`](https://docs.rs/magic-crypt/latest/magic_crypt/) crate for that.

:bulb: You only have to setup the `PIPPO_CRYPTKEY`, no `pippo.json` required for encrypting / decrypting credentials!

```bash
$ PIPPO_CRYPTKEY='foo!$bar' pippo encrypt "hello world"
8cLHS/BXGOG60nOQnYOpow==
$ PIPPO_CRYPTKEY='foo!$bar' pippo decrypt 8cLHS/BXGOG60nOQnYOpow==
hello world
```

You can then use the following format to use an encrypted variable in your YAML config:
```yaml
programs:
  - id: 12345
    environments:
      - id: 67890
        variables:
          - name: secret
            value: $enc 8cLHS/BXGOG60nOQnYOpow==
            type: secretString
```

Prefixing the value with `$enc` tells pippo that this value is encrypted. It will decrypt it on runtime and push the
decrypted value to Cloud Manager.
> ⚠ You can only use `$enc` with variables of type `secretString`. Using `string` variables will always render the value
> in plain text.

### Pipelines

* List all pipelines of the specified program (**GET** /api/program/{program_id}/pipelines)
* List pipeline variables of the specified pipeline (**GET** /api/program/{program_id}/pipeline/{pipeline_id}/variables)
* Invalidate pipeline cache of a specified pipeline (**DELETE** /api/program/{program_id}/pipeline/{pipeline_id}/cache)
* Set pipeline variables via YAML input (**PATCH** /api/program/{program_id}/pipeline/{pipeline_id}/variables)
* Execute a pipeline of a specified program (**PUT** /api/program/{program_id}/pipeline/{pipeline_id}/execution)
* List last 20 executions of a pipeline of a specified program (**GET** /api/program/{program_id}/pipeline/{pipeline_id}/executions)


ℹ️ It is possible to pass the program ID by setting the environment variable `PIPPO_PROGRAM_ID`.
ℹ️ It is possible to pass the pipeline ID by setting the environment variable `PIPPO_PIPELINE_ID`.

#### Example usage

```bash
pippo -c <pippo.json> -p <program-id> pipeline list
pippo -c <pippo.json> -p <program-id> -i <pipeline-id> pipeline vars list
pippo -c <pippo.json> pipeline vars set <FILE>
pippo -c <pippo.json> -p <program-id>  -i <pipeline-id> pipeline run
pippo -c <pippo.json> -p <program-id>  -i <pipeline-id> pipeline list-executions
pippo -c <pippo.json> -p <program-id>  -i <pipeline-id> pipeline invalidate-cache
```

To set pipeline variables given a YAML file with the format below, run
```bash
pippo -c <pippo.json> pipeline vars set <pipeline-variables.yml>
```

ℹ️ In CI environments it is recommended to run `pipeline vars set` in CI mode e.g. `pipeline vars set --ci`. See also [CI Mode](#ci-mode).

```yaml
---
programs:
  - id: 56712
    pipelines:
      - id: 7654321
        variables:
          - name: FOO
            value: bar
            type: string
          - name: SECRET_FOO
            value: $enc muchEncryptedString
            type: secretString
```

### Logs

* Download a specific logfile (**GET** /api/program/{program_id}/environment/{env_id}/logs/download)
* Tail a specific logfile (**GET** /api/program/{program_id}/environment/{env_id}/logs/download)

#### Example usage

```bash
pippo -c <pippo.json> -p <program-id> -e <environment-id> log save --service <svc> --log <log> --date <YYYY-MM-DD>
pippo -c <pippo.json> -p <program-id> -e <environment-id> log tail --service <svc> --log <log>
```

### dry-run mode

You can pass the flag `--dry-run` on the command line to preview the changes for
* environment variables
* pipeline variables

### CI mode

Since updating running pipelines or environments that are currently updating is not possible pippo will normally wait until it is possible.

To avoid long running jobs in the CI which is waiting for minutes or even hours for a pipeline there is a "ci mode" which skips the update of not updatable resources.

This mode is currently implementes for:
* pipeline variables
* environment variables

#### Example usage

```bash
pippo -c <pippo.json> pipeline vars set --ci <FILE>
pippo -c <pippo.json> env vars set --ci <environment-variables.yml>
```

### Domains

* List all Domains (**GET** /api/program/{programId}/domainNames)
* Create Domains from environment.yml (**POST** /api/program/{programId}/domainNames)

The current state of implementation is only creating new domains. It will not update nor delete other domains.
In case a domain is already there, error ALREADY_IN_USE is shown.

#### List arguments

You can provide `--start` and limit `--limit` to `domain list` in order to limit / page the results.

#### Example Data

```yaml
---
programs:
  - id: 56712
    environments:
      - id: 7654321
        domains: 
          - domainname: some.domain.de
            certificateId: 2345
```

 #### Example usage

```bash
pippo -c <pippo.json> -p <program-id> domain list
pippo -c <pippo.json> -p <program-id> domain list --start 0 --limit 20 
pippo -c <pippo.json> -p <program-id> domain list --start 20 --limit 20
pippo -c <pippo.json> domain create <environment-domains.yml>
```

### Certificates

* List all certificates (**GET** /api/program/{programId}/certificates)
* Create certificates from certificates.yaml (**POST** /api/program/{programId}/certificates )
* Update certificates from certificates.yaml (**PUT** /api/program/{programId}/certificate/{certificateId} )

#### Prerequisites:

* Certificate data (certificate, chain and key) need to present unencrypted in the local filesystem
  * There is no tooling available that is requesting new certificate data from your CA. 
* Certificates in chain are in correct order (pippo is currently not sorting them)
* Certificate paths in the `certificates.yaml` need to be relative to the `certificates.yaml` file. 

#### Create/update flow

Pippo will read and parse the provided `certificates.yaml` and will then perform a preflight check which checks if all certificate files in the `certificate.yaml` do exist.

Afterwards pippo will iterate over all certificates and perform the following steps:
* Check for existing certificate in CloudManager based on name and/or id
  * When existing certificate is found it will check if the serial number differs
    * If yes the existing certificate will be updated
    * If not, the existing certificate will not be updated
  * When no existing certificate is found a new certificate will be created
  * In case of any issues the issues are logged and after all certificates are handled the program will either exit with success or an error code



#### Example Data

```yaml
---
programs:
  - id: 56712
    certificates: 
      - name: prod-cert
        certificate: prod/prod-cert.pem
        chain: prod/prod-cert-chain.pem
        key: prod/prod-cert-key.pem
      - name: stage-cert
        certificate: stage/prod-cert.pem
        chain: stage/prod-cert-chain.pem
        key: stage/prod-cert-key.pem
            
```

#### Example usage

```bash
pippo -c <pippo.json> -p <program-id> certificates list
pippo -c <pippo.json> -p <program-id> certificates manage <path-to-certificates.yaml>  
```

## Development

## Install Rust and Cargo

On Linux and macOS systems, this is done as follows:

```bash
curl https://sh.rustup.rs -sSf | sh
```

For other OS's check https://rustup.rs/


### Run application

```bash
cargo run env vars set <path>
cargo run pipeline vars set <path>
# ...
```

### Set log level

Possible values:

* error
* warn
* info
* debug
* trace

```bash
export RUST_LOG="debug" # or
RUST_LOG="trace"; cargo run
```
### Debugging communication

Example using mitmproxy.
Update `src/main.rs` with:

```
//const HOST_NAME: &str = "https://cloudmanager.adobe.io";
const HOST_NAME: &str = "http://localhost:8443";
```

```
mitmproxy \
  --mode reverse:https://cloudmanager.adobe.io \
  --listen-host 127.0.0.1 \
  --listen-port 8443
``` 
 
Compile and run the application. You should see the communication in mitmproxy.

### Apply formatting

```bash
# apply
cargo fmt
# check only
cargo fmt --check
```

## Release

1. Goto the releases page, create and publish a new release with a version number following [semantic versioning](https://semver.org/).
2. Wait until the release workflow is complete, verify that [release workflow](https://github.com/wcm-io-devops/pippo/actions/workflows/release.yml) finished successfully.
3. Validate that binaries were correctly attached to the new release
4. Ensure that changelist is correct.