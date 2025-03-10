### Environment Variables
Before running the scripts, you need to set the required environment variables. These variables configure domains, keys, and other parameters.

List of Variables:

- `MNEMONIC` — The mnemonic phrase used for wallet operations.
- `WALLET_VERSION` — The wallet version (e.g., `v4`).
- `API_KEY` — The API key for external services.
- `DISPATCH_DOMAIN` — The source domain for message dispatch.
- `TARGET_DOMAIN` — The target domain for message delivery.
- `TON_ADDRESS` — The TON network address for refund purposes.

### Description for variables:
#### 🔑 `MNEMONIC`
The mnemonic phrase used for wallet operations.

#### 💾 `WALLET_VERSION`
The wallet version (e.g., `v4`).

#### 🔐 `API_KEY`
The API key for external services.

#### 🌐 `DISPATCH_DOMAIN`
The source domain for message dispatch.

#### 📨 `TARGET_DOMAIN`
The target domain for message delivery.

#### 🏦 `TON_ADDRESS`
The TON network address for refund purposes.

### Example of Setting Environment Variables
You can set the variables directly in your shell:

```shell
export MNEMONIC="your-mnemonic-phrase"
export WALLET_VERSION="v4"
export API_KEY="your-api-key"
export DISPATCH_DOMAIN="777001"
export TARGET_DOMAIN="777002"
export TON_ADDRESS="EQC2..."
```
### Also you need choose run-locally test mod. The TEST_CASE variable in the environment is responsible for this:
- `ton_to_ton` - running a test environment between Ton and Ton, sending a message normally
- `ton_to_evm` - running a test environment between Ton and EVM (for example, aribtrum sepolia), sending a message normally
- `ton_warp_route` - launching a warp route between Ton and Ton. The first thing that happens is the native Ton is sent, and synTon is released. The synTon is then burned and the native Ton is unlocked. 


### To run tests, for example use the following command:
```bash
TEST_CASE=ton_warp_route RUST_LOG=info cargo test --release --package run-locally --bin run-locally --features ton test-utils -- ton::test --nocapture > logs.log
```




