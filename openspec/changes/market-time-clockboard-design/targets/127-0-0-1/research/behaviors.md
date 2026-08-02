# Behaviors: primary Mark Time page

- Time-driven: UTC/local numerals and the cursor update once per second; service segments do not
  animate or change until a new response arrives.
- Input: UTC date selection queries the server at 12:00Z; previous/next only edits the native date
  input until the user selects 查看.
- Live: refresh at the earliest supplied segment end, capped at one minute.
- Error: retain a successful snapshot as stale; first-load failure has a retry action.
- Responsive: below 640px each row stacks without page-level horizontal overflow.
- Asset failure: external map/font loss leaves the complete semantic timetable intact.
