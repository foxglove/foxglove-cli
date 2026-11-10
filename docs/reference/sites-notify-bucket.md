<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sites notify-bucket`

[Command reference](README.md) · [Parent command](sites.md)

```text
Notify a Primary Site that an object in its bucket was created or removed. Most users should have their storage provider send these notifications automatically instead. This request authenticates with a Site Bucket Notification Token (fox_snt_...), not your login: pass it with --token or FOXGLOVE_SITE_NOTIFICATION_TOKEN.

Usage: foxglove sites notify-bucket [OPTIONS] --message-id <MESSAGE_ID> --file-name <FILE_NAME> --bucket-name <BUCKET_NAME> --storage-provider <STORAGE_PROVIDER>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --message-id <MESSAGE_ID>
          Unique message ID; object-created notifications are deduplicated on it

      --config <CONFIG>
          Config file

      --file-name <FILE_NAME>
          Name of the object in the bucket

      --bucket-name <BUCKET_NAME>
          Bucket name

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --storage-provider <STORAGE_PROVIDER>
          Storage provider of the bucket; must match the site's inbox or indexed bucket

          [possible values: google_cloud, aws, azure, s3_compatible]

      --azure-storage-account-name <AZURE_STORAGE_ACCOUNT_NAME>
          Azure storage account that contains the container; only valid with --storage-provider azure, and required there for bring-your-own-storage sites

      --notification-type <NOTIFICATION_TYPE>
          Kind of object event (default: object_created)

          [possible values: object_created, object_removed]

      --device-id <DEVICE_ID>
          Device to associate with the recording (object_created only)

      --device-name <DEVICE_NAME>
          Name of the device to associate with the recording; created if it does not exist (object_created only)

      --project-id <PROJECT_ID>
          Project to associate with the recording (object_created only)

      --session-id <SESSION_ID>
          Existing session to associate with the recording (object_created only)

      --session-key <SESSION_KEY>
          Key of a session to associate with the recording; ignored with --session-id (object_created only)

      --token <TOKEN>
          Site Bucket Notification Token (default: FOXGLOVE_SITE_NOTIFICATION_TOKEN)

  -h, --help
          Print help (see a summary with '-h')
```
