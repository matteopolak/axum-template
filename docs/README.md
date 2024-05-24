## Authentication

Authentication to the API is supported by two mechanisms: API keys and session tokens.

### API Keys

API keys are used to authenticate automated requests on behalf of a user.
They are passed in the `Authorization` header in the following format:

```
Authorization: Bearer <Key>
```

### Session Tokens

Session tokens are used to authenticate users.
They are passed as a cookie in the following format:

```
Cookie: session=<SessionId>
```

## Error Handling

All 400-level errors are guaranteed to have a JSON body with the following structure
for consistency and i18n support:

```json
[
  {
    "code": "some_error_code",
    "message": "Optional message describing the error.",
    "details": {
      "id": "00000000-0000-0000-0000-000000000000"
    }
  }
]
```

