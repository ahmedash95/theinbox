# Migration Strategy: High-Performance Email Access

**Objective:** Avoid UI-dependent methods and complex OAuth flows by using direct IMAP connections with app passwords.

**Target Systems:** Gmail (Remote IMAP only).

---

## 1. Authentication Strategy: "App Passwords"

**Status:** `Recommended`
**Motivation:** Eliminates the need for browser redirection, token refreshing, and Google Cloud Project management.

* **Protocol:** IMAP/SMTP over SSL.
* **Credential Type:** Google App Password (16-char string).
* **Configuration:**
* **Host:** `imap.gmail.com`
* **Port:** `993`
* **User:** Full email address.
* **Auth:** Direct string injection (bypass OAuth headers).



> **Agent Note:** If authentication fails, prompt the user to enable 2FA and generate an App Password via `myaccount.google.com/apppasswords`.

---

## 2. Remote Access: Fast Batch Operations (IMAP)

**Status:** `Active`
**Optimization:** Replace single-message iteration with vector/batch operations.

### Workflow: List and Mark-Read

1. **Search Phase:** Retrieve only `UIDs` using the `UNSEEN` flag. Do not fetch headers or bodies yet.
2. **Vector Operation:** Use the resulting list of IDs to send a single `STORE` command.
3. **Fetch Phase (Optional):** Retrieve headers only (`RFC822.HEADER`) if summarization is required.

### Implementation Pattern (Python)

```python
import imaplib

def execute_batch_mark_read(username, app_password):
    with imaplib.IMAP4_SSL("imap.gmail.com") as mail:
        mail.login(username, app_password)
        mail.select("inbox")
        
        # 1. Search (Returns minimal data: IDs)
        status, messages = mail.search(None, "UNSEEN")
        email_ids = messages[0].split()
        
        if not email_ids:
            return {"status": "success", "count": 0}

        # 2. Batch Update (O(1) network request vs O(n))
        # Join IDs with comma: b"1,2,3,4"
        batch_ids = b",".join(email_ids)
        mail.store(batch_ids, "+FLAGS", "\\Seen")
        
        return {"status": "success", "count": len(email_ids)}

```

---

## 3. Scope Note

This project is intentionally IMAP-only and does not access local Mail.app databases or AppleScript.
