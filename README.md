# Sync2Bucket

A modern desktop application for syncing folders with Scaleway S3 cloud storage. Built with Tauri 2.x, Next.js 14, and Rust.

![Sync2Bucket](src-tauri/icons/128x128.png)

## Features

- 📁 **Two-way Sync**: Upload local folders to cloud or download cloud folders locally
- 🔐 **Encrypted License Keys**: Secure user identification with AES-256-GCM encryption
- 📊 **Real-time Progress**: Live progress tracking with speed and ETA
- ⏸️ **Pause/Resume**: Interrupt and resume sync operations
- 🗑️ **Cloud Management**: Delete all cloud files with confirmation
- ⏰ **Expiration Alerts**: Warns users about upcoming credential expiration
- 🛡️ **Admin Controls**: Whitelist/blacklist system for key management

## Tech Stack

- **Desktop Shell**: [Tauri 2.x](https://tauri.app/) (Rust)
- **Frontend**: Next.js 14 + TypeScript + Tailwind CSS
- **State Management**: Zustand
- **Animations**: Framer Motion
- **S3 Client**: rusoto_s3 (Rust)
- **Encryption**: AES-256-GCM

## Project Structure

```
sync2bucket/
├── src/                      # Next.js frontend
│   ├── app/                  # App router pages
│   ├── components/           # React components
│   │   ├── KeyEntry.tsx      # License key input
│   │   ├── SyncPanel.tsx     # Main sync interface
│   │   └── Progress.tsx      # Progress display
│   └── lib/                  # Utilities & store
├── src-tauri/                # Tauri/Rust backend
│   ├── src/
│   │   ├── lib.rs            # Tauri app setup
│   │   ├── commands.rs       # IPC command handlers
│   │   ├── s3_client.rs      # S3 operations
│   │   ├── sync_engine.rs    # Sync logic
│   │   ├── crypto.rs         # Key encryption/decryption
│   │   ├── admin.rs          # Whitelist/blacklist system
│   │   ├── secrets.rs        # ⚠️ NOT IN GIT - credentials
│   │   └── bin/keygen.rs     # Key generator CLI
│   └── icons/                # App icons
└── package.json
```

## Development Setup

### Prerequisites

- **Node.js** 18+
- **Rust** 1.70+
- **macOS** 10.15+ (for building macOS apps)

### 1. Clone and Install Dependencies

```bash
git clone <repository-url>
cd sync2bucket
npm install
```

### 2. Configure Secrets

Copy the secrets template and fill in your values:

```bash
cp src-tauri/src/secrets.example.rs src-tauri/src/secrets.rs
```

Edit `src-tauri/src/secrets.rs`:

```rust
// Scaleway S3 credentials
pub const S3_ACCESS_KEY: &str = "YOUR_SCALEWAY_ACCESS_KEY";
pub const S3_SECRET_KEY: &str = "YOUR_SCALEWAY_SECRET_KEY";

// Master key for license encryption (exactly 32 bytes)
pub const MASTER_ENCRYPTION_KEY: &[u8; 32] = b"YOUR_32_CHARACTER_SECRET_KEY!!!";
```

> ⚠️ **IMPORTANT**: Never commit `secrets.rs` to version control!

### 3. Development Mode

```bash
npm run tauri:dev
```

This starts the Next.js dev server and opens the Tauri app in development mode.

### 4. Build for Production

```bash
npm run tauri:build
```

Output files:
- `src-tauri/target/release/bundle/macos/Sync2Bucket.app`
- `src-tauri/target/release/bundle/dmg/Sync2Bucket_x.x.x_aarch64.dmg`

## Code Signing & Notarization (macOS)

### Code Signing

The app is configured to sign with a Developer ID certificate. Ensure you have:

1. An Apple Developer account ($99/year)
2. A "Developer ID Application" certificate installed

The signing identity is configured in `src-tauri/tauri.conf.json`:

```json
"macOS": {
  "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)"
}
```

### Notarization

For Gatekeeper approval (no security warnings), set environment variables before building:

```bash
export APPLE_ID="your-apple-id@email.com"
export APPLE_PASSWORD="xxxx-xxxx-xxxx-xxxx"  # App-specific password
export APPLE_TEAM_ID="YOUR_TEAM_ID"

npm run tauri:build
```

To create an app-specific password:
1. Go to https://appleid.apple.com
2. Sign in → Security → App-Specific Passwords → Generate

## Generating User License Keys

### Using the CLI Tool

After building, use the `keygen` binary:

```bash
cd src-tauri
./target/release/keygen --name "User Name"
```

Output:
```
╔══════════════════════════════════════════════════════════════╗
║                    SYNC2BUCKET LICENSE KEY                    ║
╠══════════════════════════════════════════════════════════════╣
║ User: User Name                                              ║
║ UID:  u_abc123def456                                         ║
║ Created: 2025-11-28 12:00:00 UTC                             ║
╠══════════════════════════════════════════════════════════════╣
║ Key:                                                         ║
║ EXAD-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ║
╚══════════════════════════════════════════════════════════════╝

Full key (copy this):
EXAD-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

### Key Format

Keys are encrypted JSON payloads containing:
- `uid`: Unique user identifier (determines cloud folder)
- `name`: User's display name
- `created`: Timestamp of key creation

Each user's files are stored in: `users/{uid}/`

## Admin Features

### Whitelist/Blacklist

Admin files are stored in the S3 bucket under `_admin/`:

- `_admin/whitelist.json` - Authorized keys
- `_admin/blacklist.json` - Disabled keys
- `_admin/activity_log.json` - User activity tracking

These files are only accessible with the master S3 credentials (not user keys).

### Activity Logging

The app logs user actions:
- Login attempts
- Sync operations
- File deletions

## Configuration

### S3 Settings

Configured in `src-tauri/src/s3_client.rs`:

```rust
const S3_ENDPOINT: &str = "https://s3.nl-ams.scw.cloud";
const S3_REGION: &str = "nl-ams";
const S3_BUCKET: &str = "cloud-storage-exad";
```

### Credentials Expiration

Set the expiration date in `src-tauri/src/s3_client.rs`:

```rust
const CREDENTIALS_EXPIRY_YEAR: i32 = 2026;
const CREDENTIALS_EXPIRY_MONTH: u32 = 11;
const CREDENTIALS_EXPIRY_DAY: u32 = 28;
```

Users will see warnings when expiration approaches.

## CI/CD - Automated Builds

The project includes GitHub Actions to automatically build for **macOS** and **Windows**.

### Setup GitHub Secrets

Go to your repo → Settings → Secrets and variables → Actions → New repository secret

Add these secrets:

| Secret | Description |
|--------|-------------|
| `S3_ACCESS_KEY` | Scaleway S3 access key |
| `S3_SECRET_KEY` | Scaleway S3 secret key |
| `MASTER_ENCRYPTION_KEY` | 32-character encryption key |
| `APPLE_ID` | Apple ID email (for notarization) |
| `APPLE_PASSWORD` | App-specific password |
| `APPLE_TEAM_ID` | Apple Team ID (e.g., V72M7CT7PD) |
| `APPLE_SIGNING_IDENTITY` | Full signing identity string |
| `APPLE_CERTIFICATE` | Base64-encoded .p12 certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Certificate password |

### Trigger a Build

**Option 1: Tag a release**
```bash
git tag v0.1.0
git push origin v0.1.0
```

**Option 2: Manual trigger**
Go to Actions → Build Sync2Bucket → Run workflow

### Build Outputs

| Platform | Artifact |
|----------|----------|
| macOS ARM64 | `Sync2Bucket.dmg` |
| macOS Intel | `Sync2Bucket.dmg` |
| Windows x64 | `Sync2Bucket.msi`, `Sync2Bucket-setup.exe` |

## Distribution

### For Users

1. Download the DMG file
2. Drag Sync2Bucket to Applications
3. Open the app and enter your license key
4. Start syncing!

### Distributing Keys

Send users:
1. The `Sync2Bucket.dmg` file
2. Their unique license key (generated with `keygen`)

## Troubleshooting

### "App can't be opened because it is from an unidentified developer"

If the app isn't notarized:
1. Right-click the app
2. Select "Open"
3. Click "Open" in the dialog

### Build Errors

1. Ensure `secrets.rs` exists with valid values
2. Run `cargo clean` in `src-tauri/` and rebuild
3. Check Rust version: `rustc --version` (requires 1.70+)

### S3 Connection Issues

1. Verify S3 credentials in `secrets.rs`
2. Check network connectivity
3. Ensure the bucket exists and is accessible

## License

Proprietary - ExAd

## Support

Contact: support@exad.com
