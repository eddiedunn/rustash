# Rustash Security Analysis

## Overall Security Posture

The Rustash project exhibits a **strong security posture**. The choice of Rust, combined with modern and idiomatic practices, eliminates many common classes of security vulnerabilities from the outset. The developers have clearly prioritized security by:

* Forbidding `unsafe` code at the workspace level.
* Using lints to deny direct panics via `.unwrap()` and `.expect()`.
* Employing a mature ORM (Diesel) which prevents SQL injection.
* Using in-process libraries (`skim`) instead of shelling out to external commands (`fzf`).

The project's main risks are in its interaction with the environment, specifically around configuration and potential denial-of-service vectors.

---

## Security Analysis by Category

### 1. Input Validation and Handling

* **Finding:** The application performs good, basic input validation for snippets. In `crates/rustash-core/src/snippet.rs`, the `validate_snippet_content` function checks for empty titles/content and enforces length limits.
* **Risk:** **Low**.
* **Assessment:** This is a solid defense against basic abuse and potential DoS attacks from excessively large inputs. The limits (255 chars for title, 100,000 for content) are reasonable.

### 2. SQL Injection

* **Finding:** The project exclusively uses the Diesel ORM's query builder for database interactions. All user-supplied data is passed as bound parameters.
* **Risk:** **Very Low**.
* **Assessment:** Diesel is designed to prevent SQL injection, and the project uses it correctly. Even the dynamic `LIKE` query in `list_snippets` is safe, as Diesel handles the parameter binding. There is no evidence of raw SQL string concatenation, which is the primary vector for this vulnerability.

### 3. Command Injection

* **Finding:** The interactive fuzzy finder is implemented using the `skim` crate (`crates/rustash-cli/src/fuzzy.rs`), which runs entirely within the Rust process.
* **Risk:** **Very Low**.
* **Assessment:** This is an excellent security choice. The initial PRP (`prp_rustash_v1.0_COMPLETED.md`) mentioned using an external `fzf` process, which would have introduced a significant command injection risk. By opting for a native Rust library, the developers have completely mitigated this threat.

### 4. Configuration and Environment Security

* **Finding:** The database path is configured via the `DATABASE_URL` environment variable in `crates/rustash-core/src/database.rs`. There is no validation or sanitization of this path.
* **Risk:** **Medium**.
* **Assessment:** This is the most significant security risk identified. A user (or a malicious script running as the user) could set this environment variable to an arbitrary path.
    * **Example Attack:** `export DATABASE_URL=~/.ssh/authorized_keys`. Running `rustash` could then corrupt this critical file.
    * **Impact:** This could lead to file corruption, denial of service (by pointing to a critical system file), or information disclosure (if the error messages leak path information).
* **Recommendation:**
    1. **Default to a Safe Location:** Instead of defaulting to `rustash.db` in the current working directory, default to a standard user config location (e.g., `~/.config/rustash/rustash.db`).
    2. **Path Validation:** If allowing a custom path, validate it. Ensure it's a `.db` file and does not point to a sensitive directory.
    3. **Documentation:** Clearly document this environment variable and the associated risks in `USER_GUIDE.md`.

### 5. Denial of Service (DoS)

* **Finding:** The `list_snippets` function uses a `LIKE` query with leading and trailing wildcards (`%...%`).
* **Risk:** **Low to Medium**.
* **Assessment:** On a large database, this query cannot use standard B-tree indexes effectively and will result in a full table scan. A malicious user could add many large snippets and then craft search queries that would consume significant CPU and time, effectively making the application unresponsive. While the `up.sql` migration sets up a Full-Text Search (FTS5) table, the `list_snippets` and `search_snippets` functions in `crates/rustash-core/src/snippet.rs` explicitly fall back to using `LIKE`.
* **Recommendation:** Modify the `search_snippets` function to correctly use the `snippets_fts` virtual table for all full-text searches. This will be significantly more performant and resistant to DoS.

### 6. Insecure Dependencies (Supply Chain)

* **Finding:** The project uses a solid set of well-maintained, popular crates. The `CLAUDE-RUST.md` and `xtask/src/main.rs` files specify the use of `cargo audit` in CI and pre-release checks.
* **Risk:** **Low**.
* **Assessment:** The practice of running `cargo audit` is a critical supply-chain security measure. This indicates a proactive approach to managing dependency vulnerabilities. The `libsqlite3-sys` crate is bundled, giving the project control over the SQLite version and patching, which is a good practice.

## Security Strengths

1. **Memory Safety:** By writing in idiomatic Rust and forbidding `unsafe` code, the project is free from entire classes of memory corruption vulnerabilities like buffer overflows and use-after-frees.
2. **Strict Linting:** The use of `clippy::pedantic` and `-Dwarnings` enforces a high standard of code quality that often correlates with better security.
3. **Panic-Free Code:** Forbidding `.unwrap()` and `.expect()` makes the application more robust and less prone to crashing (a form of DoS). The use of `anyhow` and `thiserror` is a best practice.
4. **Secure-by-Default Libraries:** The choice of Diesel (prevents SQLi) and `skim` (prevents command injection) shows a mature approach to selecting dependencies.
5. **Automated Security Checks:** Integrating `cargo audit` into the development lifecycle is a major strength.

## Summary and Recommendations

| Priority | Risk Category             | Recommendation |
|----------|---------------------------|----------------------------------------------------------------------------------------------------------------------------------|
| **High** | Configuration Security    | **Validate `DATABASE_URL`**. Change the default to a safe user-specific directory (e.g., `~/.config/rustash/`) and validate any custom paths. |
| **Medium** | Denial of Service         | **Implement FTS5 search**. Refactor `search_snippets` to use the existing `snippets_fts` table instead of `LIKE` for full-text queries. |
| **Low**  | Documentation/User Safety | **Update `USER_GUIDE.md`**. Add a section warning users about pasting and executing malicious commands from untrusted snippets. |

## Reporting Security Issues

If you discover a security issue in Rustash, please report it by opening an issue on our GitHub repository. For sensitive security issues, please contact the maintainers directly.

## Security Updates

Security updates are handled through regular version releases. It is recommended to always use the latest version of Rustash to ensure you have all security patches.
