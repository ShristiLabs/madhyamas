# Backup and Disaster Recovery

The licensing server stores two categories of critical data:

1. **PostgreSQL database** — accounts, customers, licenses, seats, audit log.
2. **Ed25519 private key** — the signing key used to issue licenses.

Both must be backed up regularly and recoverable in case of data loss.

## PostgreSQL Backup

### pg_dump (logical backup)

```sh
# Full backup (compressed)
pg_dump "postgres://madhyamas:password@localhost:5432/madhyamas" \
    --format=custom --file=madhyamas_$(date +%Y%m%d).dump

# Restore
pg_restore --clean --if-exists \
    --dbname="postgres://madhyamas:password@localhost:5432/madhyamas" \
    madhyamas_20260101.dump
```

### Automated daily backup (cron)

```sh
# /etc/cron.d/madhyamas-licensing-backup
0 2 * * * postgres pg_dump "postgres://madhyamas:password@localhost:5432/madhyamas" \
    --format=custom --file=/backups/madhyamas_$(date +\%Y\%m\%d).dump && \
    find /backups -name "madhyamas_*.dump" -mtime +30 -delete
```

This keeps 30 days of backups. Adjust retention based on your requirements.

### Point-in-time recovery (PITR)

For production, enable PostgreSQL's Write-Ahead Log (WAL) archiving for
point-in-time recovery:

```conf
# postgresql.conf
archive_mode = on
archive_command = 'aws s3 cp %p s3://my-bucket/wal/%f'
wal_level = replica
```

With WAL archiving, you can restore the database to any point in time:

```sh
# Restore base backup, then replay WAL up to a target time
pg_restore --dbname=postgres base_backup.dump
recovery_target_time = '2026-01-15 14:30:00'
```

### Cloud-managed PostgreSQL

If using AWS RDS, Google Cloud SQL, or Azure Database for PostgreSQL, use
the managed backup features:

- **AWS RDS:** Automated daily snapshots, point-in-time recovery to any
  second within the retention period (default 7 days, max 35 days).
- **Google Cloud SQL:** Automated backups with 7-day retention, PITR.
- **Azure Database for PostgreSQL:** Automatic backups with 7-35 day retention.

## Ed25519 Key Backup

The private key is irreplaceable — if lost, all licenses signed by it cannot
be re-issued without a new key (which requires rotating the public key on all
proxy instances).

### Backup strategy

1. **Primary copy:** Secrets manager (AWS Secrets Manager, HashiCorp Vault).
2. **Secondary copy:** Offline storage (USB drive in a safe, HSM).
3. **Tertiary copy:** Another secrets manager region/replica.

Store the key in at least two geographically separated locations. Test
recovery from each backup.

### Key recovery

```sh
# From AWS Secrets Manager
aws secretsmanager get-secret-value \
    --secret-id madhyamas/ed25519-private-key \
    --query SecretBinary --output text | base64 --decode > /keys/ed25519_private.key

# From HashiCorp Vault
vault kv get -field=private_key secret/madhyamas/ed25519 > /keys/ed25519_private.key
```

## License Data Backup

The `licenses` table is the **source of truth** for all issued licenses. It
contains:

- The license claims (customer, plan, seats, features, expiry).
- The Ed25519 signature (so licenses can be re-downloaded without re-signing).
- The license status (active, suspended, revoked).

### What to back up

| Table | Importance | Backup frequency |
|---|---|---|
| `licenses` | Critical — source of truth | Daily (or continuous with PITR) |
| `accounts` | Critical — customer data | Daily |
| `customers` | Critical — customer data | Daily |
| `seats` | Important — seat tracking | Daily |
| `audit_log` | Important — compliance | Daily |

### Recovery procedure

1. Restore the PostgreSQL database from the latest backup.
2. Verify the Ed25519 private key is accessible (from secrets manager).
3. Start the licensing server.
4. Verify a sample license by calling `POST /api/licenses/verify`.
5. Check the audit log for the last recorded event.

## Disaster recovery plan

### Scenario: Database loss

1. Provision a new PostgreSQL instance.
2. Restore from the latest backup (`pg_restore` or cloud-managed restore).
3. Update the licensing server's `DATABASE_URL` to point to the new instance.
4. Restart the licensing server.
5. Verify license issuance and verification work.

### Scenario: Private key loss

1. Generate a new Ed25519 keypair.
2. Update the licensing server to use the new private key.
3. Re-sign all active licenses (query the `licenses` table, re-sign each).
4. Distribute the new public key to all proxy instances.
5. Revoke the old key (it's lost, but licenses signed with it are now
   unverifiable — customers must download new license files).

### Scenario: Complete server loss

1. Provision new infrastructure (Docker host or Kubernetes cluster).
2. Deploy the licensing server (using the Dockerfile or K8s manifests).
3. Restore PostgreSQL from backup.
4. Restore the Ed25519 private key from secrets manager backup.
5. Verify the server is operational.

### RTO and RPO

| Metric | Target | How |
|---|---|---|
| RTO (Recovery Time Objective) | < 4 hours | Cloud-managed PostgreSQL + containerized server |
| RPO (Recovery Point Objective) | < 24 hours | Daily pg_dump or PITR with < 1 hour WAL archive |
