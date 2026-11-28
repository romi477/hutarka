#!/bin/sh
# Creates the self-signed code-signing certificate "Hutarka Local" in the login
# keychain. Run once, by hand: macOS asks for the keychain password.
#
# A stable identity keeps macOS permissions (Accessibility, Microphone) across
# rebuilds; ad-hoc signing loses them on every build.
set -eu

NAME="Hutarka Local"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

# The same check as scripts/app.sh: a certificate that can sign (its private
# key is there and it is trusted for code signing).
if security find-identity -v -p codesigning | grep -q "\"$NAME\""; then
    echo "\"$NAME\" can already sign code. Nothing to do."
    exit 0
fi
if security find-certificate -c "$NAME" "$KEYCHAIN" >/dev/null 2>&1; then
    echo "A certificate named \"$NAME\" exists but cannot sign code (no private key, or not trusted)." >&2
    echo "Open Keychain Access, delete every \"$NAME\" certificate and key, then run this script again." >&2
    exit 1
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

cat >"$TMP/cert.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
EOF

openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
    -keyout "$TMP/key.pem" -out "$TMP/cert.pem" -config "$TMP/cert.cnf"

# A random one-time password: macOS `security` refuses an empty one.
PASS=$(openssl rand -hex 16)
openssl pkcs12 -export -legacy -inkey "$TMP/key.pem" -in "$TMP/cert.pem" \
    -name "$NAME" -out "$TMP/cert.p12" -passout "pass:$PASS" 2>/dev/null \
    || openssl pkcs12 -export -inkey "$TMP/key.pem" -in "$TMP/cert.pem" \
        -name "$NAME" -out "$TMP/cert.p12" -passout "pass:$PASS"

security import "$TMP/cert.p12" -k "$KEYCHAIN" -P "$PASS" -T /usr/bin/codesign

# Trust it for code signing (asks for your login password).
security add-trusted-cert -p codeSign -k "$KEYCHAIN" "$TMP/cert.pem"

echo
security find-identity -v -p codesigning | grep "$NAME" \
    && echo "Done: \"$NAME\" can sign code." \
    || echo "Imported, but not listed as valid yet: open Keychain Access, find \"$NAME\", Trust > Code Signing: Always Trust."
