# BlinkTicket Smart Contract Security Audit

**Contract:** `EventTickets` (Anchor/Rust)
**Program ID:** `EFzK4HY7f8yr9qqsMJcPunCTwHF9cA69h265UR58bvj1`
**Audited:** 2026-03-22
**Auditor:** Automated review + manual analysis

---

## Findings Summary

| # | Severity | Title | Status |
|---|----------|-------|--------|
| 1 | CRITICAL | Loyalty discount spoofable via client-provided `loyalty_count` | FIXED |
| 2 | HIGH | `tickets_sold -= 1` underflow in `cancel_ticket` | FIXED |
| 3 | MEDIUM | `tickets_sold += 1` overflow in `buy_ticket` | FIXED |
| 4 | MEDIUM | SOL transfer before state update (CEI violation) in `buy_ticket` | FIXED |
| 5 | MEDIUM | `checked_mul`/`checked_div` use `.unwrap()` instead of error propagation | FIXED |
| 6 | LOW | No explicit `is_active` check before `update_event` | ACCEPTED |
| 7 | INFO | All PDA seeds are unique and non-colliding | OK |
| 8 | INFO | All access control checks via `has_one` and `Signer` are correct | OK |
| 9 | INFO | All input validation (strings, ratings, event_type) is present | OK |

---

## Finding Details

### 1. CRITICAL — Loyalty discount spoofable

**Before:** `buy_ticket` accepted `loyalty_count: u8` as a client-provided argument. Any user could pass `loyalty_count = 3` to receive a 20% discount without holding any AttendanceRecord POAPs.

**Fix:** Removed `loyalty_count` parameter. The program now verifies loyalty on-chain by iterating `remaining_accounts` and deserializing each as `AttendanceRecord`, checking that `record.attendee == buyer` and `record.authority == event.authority`. Only valid, matching records count toward the threshold.

**Impact:** Eliminated economic attack vector where users could steal 20% of every ticket price.

### 2. HIGH — `tickets_sold` underflow in `cancel_ticket`

**Before:** `event.tickets_sold -= 1` — raw subtraction on `u16`. If called when `tickets_sold == 0` (theoretically possible if ticket PDA exists from a prior program version), this would wrap to `u16::MAX` (65535).

**Fix:** Changed to `event.tickets_sold.checked_sub(1).ok_or(EventError::ArithmeticOverflow)?`

### 3. MEDIUM — `tickets_sold` overflow in `buy_ticket`

**Before:** `event.tickets_sold += 1` — raw addition. At `u16::MAX` this wraps to 0.

**Fix:** Changed to `event.tickets_sold.checked_add(1).ok_or(EventError::ArithmeticOverflow)?`

### 4. MEDIUM — CEI violation in `buy_ticket`

**Before:** SOL transfer via `invoke()` happened on lines 102-114, *before* ticket state was written on lines 117-124. While Solana's runtime prevents true reentrancy (accounts are locked during CPI), the Checks-Effects-Interactions pattern should still be followed as a defense-in-depth measure.

**Fix:** Moved all state updates (ticket fields + `tickets_sold` increment) BEFORE the SOL transfer instruction.

### 5. MEDIUM — `.unwrap()` on checked math

**Before:** `checked_mul(...).unwrap()`, `checked_div(...).unwrap()`, `checked_sub(...).unwrap()` in the loyalty discount calculation would panic and halt the program with an opaque error.

**Fix:** Replaced all `.unwrap()` with `.ok_or(EventError::ArithmeticOverflow)?` for proper error propagation.

### 6. LOW — No `is_active` check on `update_event`

**Status:** ACCEPTED. The authority may want to update an inactive event (e.g., reactivate it). This is a design choice, not a vulnerability.

### 7. INFO — PDA seed analysis

All PDA seeds are unique and cannot collide:
- **Event:** `[authority, "event", event_id_le_bytes]` — unique per authority + event_id
- **Ticket:** `[event_pda, "ticket", buyer]` — unique per event + buyer (one ticket per buyer per event)
- **AttendanceRecord:** `[attendee, "poap", event_pda]` — unique per attendee + event
- **Review:** `[event_pda, "review", reviewer]` — unique per event + reviewer

### 8. INFO — Access control

All access control is correctly enforced:
- `update_event`: `has_one = authority` + `authority: Signer`
- `close_event`: `has_one = authority` + `authority: Signer`
- `issue_poap`: `has_one = authority` + `authority: Signer`
- `cancel_ticket`: `has_one = owner` + `owner: Signer`
- `leave_review`: `has_one = owner` + `constraint = ticket.owner == reviewer.key()` + `reviewer: Signer`
- `buy_ticket`: `event_authority` validated via `constraint = event_authority.key() == event.authority`

### 9. INFO — Input validation

All user inputs are validated:
- `name.len() <= 50`, `description.len() <= 200`
- `max_tickets > 0`
- `event_type <= 4`
- `rating >= 1 && rating <= 5`
- `comment.len() <= 280`

---

## Architecture Notes

- `buy_ticket` no longer takes any arguments — loyalty is verified purely on-chain
- Clients that want loyalty discounts must pass AttendanceRecord account infos as `remaining_accounts`
- Added `ArithmeticOverflow` error variant for all checked math operations
