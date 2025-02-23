# mqtt-notify-rs

mqtt-notify-rs is a Rust-based notification bridge that subscribes to an MQTT
topic and forwards incoming messages as notifications to one or more channels.

## Notifiers

- **Desktop:** Utilizes the system's native notification mechanism.
- **XMPP:** Sends messages to a specified XMPP recipient.

## Usage

```
Usage: mqtt-notify-rs [OPTIONS] <MQTT_URL>

Arguments:
  <MQTT_URL>  MQTT URL (mqtt[s]://[user@]host[:port][/topic])

Options:
      --desktop                  Enable desktop notifications
      --xmpp <RECIPIENT>         Enable XMPP notifications (can be specified multiple times)
      --xmpp-credentials <FILE>  Path to the XMPP credentials file [default: ~/.sendxmpprc]
  -h, --help                     Print help
  -V, --version                  Print version
```

For example:

```sh
$ export MQTT_PASSWORD=MySecretPassword
$ mqtt-notify-rs mqtts://user@mqtt.example.com \
  --desktop \
  --xmpp me@example.com
```

## Configuration Details

### MQTT URL

The MQTT URL should follow this format:

```
mqtt[s]://[user@]host[:port][/topic]
```

- **Scheme:**
  Use `mqtt` for unencrypted connections or `mqtts` for encrypted ones.
- **User Info:**
  If a user is provided, password will be retrieved from `MQTT_PASSWORD`
  environment variable or with an interactive prompt.
- **Topic:**
  The path component defines the topic. If omitted the default topic
  `notifications` is used.

### XMPP Credentials

For XMPP notifications, the credentials file (default: `~/.sendxmpprc`) should
contain your XMPP JID and password on a single line separated by space.

## License

This project is licensed under the MIT License.
