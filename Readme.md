# WordWeaver Backend (at the moment only)

## Start DB

```bash
surreal start --user root --pass root memory
```

## Start server

```bash
run --package WordWeaverBackend --bin WordWeaverBackend
```

## Useful resources

- https://github.com/scotow/cobrust/blob/master/src/lobby/mod.rs#L16
- https://github.com/tokio-rs/axum/blob/main/examples/chat/src/main.rs
- https://stackblitz.com/edit/sb1-mutlfj?file=src%2FApp.tsx
- https://bolt.new/~/sb1-mutlfj