#!/usr/bin/env python3
"""Independent AWS SigV4 reference implementation.

Written straight from the AWS "Signature Version 4 signing process"
specification using only the standard library, with no reference to the Rust
code in `src/embedding/aws/sigv4.rs`. Its output is committed to
`sigv4_bedrock_expected.txt` and asserted by the Rust unit tests, so the two
implementations check each other.

Regenerate with:

    python3 crates/maproom/tests/fixtures/sigv4_reference.py \
        > crates/maproom/tests/fixtures/sigv4_bedrock_expected.txt
"""

import hashlib
import hmac

ACCESS_KEY = "AKIDEXAMPLE"
SECRET_KEY = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY"
REGION = "us-east-1"
SERVICE = "bedrock"
TIMESTAMP = "20150830T123600Z"
DATE = TIMESTAMP[:8]

HOST = "bedrock-runtime.us-east-1.amazonaws.com"
METHOD = "POST"
# The path as it appears on the wire: the model id's ':' is already encoded.
WIRE_PATH = "/model/amazon.titan-embed-text-v2%3A0/invoke"
BODY = b'{"inputText":"hello"}'

UNRESERVED = set(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ" "abcdefghijklmnopqrstuvwxyz" "0123456789" "-_.~"
)


def uri_encode(segment):
    """Percent-encode everything outside RFC 3986's unreserved set."""
    out = []
    for byte in segment.encode("utf-8"):
        char = chr(byte)
        out.append(char if char in UNRESERVED else "%%%02X" % byte)
    return "".join(out)


def canonical_uri(path):
    """Non-S3 services encode each path segment a second time."""
    return "/".join(uri_encode(seg) for seg in path.split("/"))


def sha256_hex(data):
    return hashlib.sha256(data).hexdigest()


def hmac_sha256(key, data):
    return hmac.new(key, data.encode("utf-8"), hashlib.sha256).digest()


def main():
    payload_hash = sha256_hex(BODY)

    headers = {
        "content-type": "application/json",
        "host": HOST,
        "x-amz-content-sha256": payload_hash,
        "x-amz-date": TIMESTAMP,
    }
    signed_headers = ";".join(sorted(headers))
    canonical_headers = "".join(
        "%s:%s\n" % (name, headers[name].strip()) for name in sorted(headers)
    )

    canonical_request = "\n".join(
        [
            METHOD,
            canonical_uri(WIRE_PATH),
            "",  # no query string
            canonical_headers,
            signed_headers,
            payload_hash,
        ]
    )

    scope = "%s/%s/%s/aws4_request" % (DATE, REGION, SERVICE)
    string_to_sign = "\n".join(
        [
            "AWS4-HMAC-SHA256",
            TIMESTAMP,
            scope,
            sha256_hex(canonical_request.encode("utf-8")),
        ]
    )

    k_date = hmac_sha256(("AWS4" + SECRET_KEY).encode("utf-8"), DATE)
    k_region = hmac_sha256(k_date, REGION)
    k_service = hmac_sha256(k_region, SERVICE)
    k_signing = hmac_sha256(k_service, "aws4_request")

    signature = hmac.new(
        k_signing, string_to_sign.encode("utf-8"), hashlib.sha256
    ).hexdigest()
    print(signature)


if __name__ == "__main__":
    main()
