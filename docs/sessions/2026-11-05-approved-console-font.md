# Approved console font detector exception

## Goal and decision

Record the design detector exception for Inter. DESIGN.md explicitly specifies Inter for the console, so the existing font is intentional. The exception applies only to the `overused-font` finding for `inter`.

## Verification

The Impeccable hook status reports only the value-specific exception, and the Windows Rust quality gate passed. The console design and application code are unchanged.
