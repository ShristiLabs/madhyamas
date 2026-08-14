# Ed25519 Key Management

The licensing server signs licenses with an Ed25519 private key. The
corresponding public key is distributed to proxy instances and used for
offline license verification. This document covers key generation, storage,
rotation, and distribution.

## Key Generation

Generate a fresh Ed25519 keypair using the licensing server's built-in
command:

```sh
madhyamas-licensing generate-keys --output-dir /path/to/keys
```

This creates two files in the output directory:

| File | Contents |
|---|---|
| `ed25519_private.key` | 32 raw bytes — the Ed25519 private signing key |
| `ed25519_public.key` | 32 raw bytes — the Ed25519 public verifying key |

The command also prints the **base64-encoded** public key, which is the value
you set as `MADHYAMAS_LICENSE_PUBLIC_KEY` on proxy instances.

### Key format

Both files contain 32 raw bytes. The licensing server's `--ed25519-private-key-file`
flag also accepts base64-encoded 32-byte strings (useful for environment
variables and secrets managers).

## Storage

### Production: secrets manager

**Never** store the private key in a config file, environment variable in
plaintext, or version control. Use a dedicated secrets manager:

#### AWS Secrets Manager

```sh
# Store the private key
aws secretsmanager create-secret \
    --name madhyamas/ed25519-private-key \
    --secret-binary file:///path/to/keys/ed25519_private.key

# Retrieve at runtime (application code or init container)
aws secretsmanager get-secret-value \
    --secret-id madhyamas/ed25519-private-key \
    --query SecretBinary --output text | base64 --decode > /keys/ed25519_private.key
```

#### HashiCorp Vault

```sh
# Store the private key
vault kv put secret/madhyamas/ed25519 \
    private_key=@/path/to/keys/ed25519_private.key \
    public_key=@/path/to/keys/ed25519_public.key

# Retrieve at runtime
vault kv get -field=private_key secret/madhyamas/ed25519 > /keys/ed25519_private.key
```

#### Kubernetes Secrets

```sh
kubectl create secret generic licensing-secrets \
    --from-file=ed25519-private-key=/path/to/keys/ed25519_private.key \
    --from-file=ed25519-public-key=/path/to/keys/ed25519_public.key \
    -n madhyamas-licensing
```

See `deploy/kubernetes/secret.yaml` for the manifest template.

### Development

For local development, omit `--ed25519-private-key-file` and the server
generates a fresh keypair on startup. This is fine for testing but licenses
signed with this key will NOT verify on proxy instances configured with a
different public key.

## Distribution

The **public key** is distributed to proxy instances via the
`MADHYAMAS_LICENSE_PUBLIC_KEY` environment variable (base64-encoded 32 bytes):

```sh
export MADHYAMAS_LICENSE_PUBLIC_KEY="$(base64 -w0 /path/to/keys/ed25519_public.key)"
madhyamas serve
```

The proxy reads this at startup and uses it for offline license verification.
If the env var is not set, the proxy falls back to a compiled-in development
key and logs a warning.

## Key Rotation

Key rotation replaces the signing key while keeping existing licenses valid
during a transition period.

### Rotation procedure

1. **Generate a new keypair:**
   ```sh
   madhyamas-licensing generate-keys --output-dir /path/to/new-keys
   ```

2. **Update the licensing server** to use the new private key (update the
   secrets manager / Kubernetes secret, then restart the server).

3. **Re-sign active licenses** with the new key. The licensing server can
   re-issue licenses for all active customers. Each re-issued license has a
   new signature but the same claims (license_id, customer, plan, seats,
   expiry, features).

4. **Distribute the new public key** to all proxy instances by updating
   `MADHYAMAS_LICENSE_PUBLIC_KEY`. Proxy instances that haven't been updated
   yet will reject licenses signed with the new key — so distribute the new
   public key BEFORE re-signing licenses, or support both keys during the
   transition.

5. **Transition period:** During rotation, the proxy can support both old and
   new public keys. This requires updating the proxy's `LicenseVerifier` to
   accept multiple keys (future enhancement). For now, rotate during a
   maintenance window.

6. **Revoke the old key** after all proxy instances have been updated and all
   active licenses have been re-signed.

### Rotation frequency

- **Routine:** Every 12-24 months.
- **After incident:** Immediately if the private key is suspected compromised.
- **Emergency:** Generate new keypair, update server, re-sign all licenses,
  distribute new public key.

## Security checklist

- [ ] Private key is stored in a secrets manager (not on disk in plaintext).
- [ ] Private key is never committed to version control.
- [ ] Private key is never logged or printed (except by the `generate-keys`
      command during initial creation).
- [ ] Access to the private key is restricted (IAM policies, RBAC).
- [ ] Public key is distributed to all proxy instances via
      `MADHYAMAS_LICENSE_PUBLIC_KEY`.
- [ ] Key rotation procedure is documented and tested.
- [ ] Backup of the private key exists in a secure, access-controlled location.
