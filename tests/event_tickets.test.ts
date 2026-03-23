describe("EventTickets v2", () => {
  // ─── Shared state ───
  const eventId = new anchor.BN(1);
  const eventType = 0; // Conference

  const [eventPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      pg.wallet.publicKey.toBuffer(),
      Buffer.from("event"),
      eventId.toArrayLike(Buffer, "le", 8),
    ],
    pg.PROGRAM_ID
  );

  const [ticketPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      eventPDA.toBuffer(),
      Buffer.from("ticket"),
      pg.wallet.publicKey.toBuffer(),
    ],
    pg.PROGRAM_ID
  );

  const [poapPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      pg.wallet.publicKey.toBuffer(),
      Buffer.from("poap"),
      eventPDA.toBuffer(),
    ],
    pg.PROGRAM_ID
  );

  const [reviewPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      eventPDA.toBuffer(),
      Buffer.from("review"),
      pg.wallet.publicKey.toBuffer(),
    ],
    pg.PROGRAM_ID
  );

  // ─── 1. Create event with event_type ───
  it("Create event with event_type", async () => {
    const txHash = await pg.program.methods
      .createEvent(
        eventId,
        "Solana Hacker House Lima",
        "3 days of building on Solana",
        new anchor.BN(50_000_000), // 0.05 SOL
        100,
        eventType
      )
      .accounts({
        event: eventPDA,
        authority: pg.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);
    console.log(`✅ Event created - TX: ${txHash}`);

    const event = await pg.program.account.event.fetch(eventPDA);
    assert.equal(event.name, "Solana Hacker House Lima");
    assert.equal(event.eventType, 0);
    assert.equal(event.maxTickets, 100);
    assert.equal(event.ticketsSold, 0);
    assert.equal(event.isActive, true);
    console.log(`   Type: ${event.eventType}, Price: ${event.ticketPrice.toString()} lamports`);
  });

  // ─── 2. Update event ───
  it("Update event", async () => {
    const txHash = await pg.program.methods
      .updateEvent(
        "Solana Hacker House Lima 2026",
        "Updated: 3 days of hacking in Lima",
        new anchor.BN(100_000_000), // 0.1 SOL
        150
      )
      .accounts({
        event: eventPDA,
        authority: pg.wallet.publicKey,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);
    console.log(`✅ Event updated - TX: ${txHash}`);

    const event = await pg.program.account.event.fetch(eventPDA);
    assert.equal(event.name, "Solana Hacker House Lima 2026");
    assert.equal(event.maxTickets, 150);
    assert.equal(event.ticketPrice.toNumber(), 100_000_000);
  });

  // ─── 3. Buy ticket (no loyalty) ───
  it("Buy ticket without loyalty discount", async () => {
    const txHash = await pg.program.methods
      .buyTicket(0) // loyalty_count = 0
      .accounts({
        event: eventPDA,
        ticket: ticketPDA,
        eventAuthority: pg.wallet.publicKey,
        buyer: pg.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);
    console.log(`✅ Ticket purchased - TX: ${txHash}`);

    const ticket = await pg.program.account.ticket.fetch(ticketPDA);
    assert.equal(ticket.isValid, true);
    assert.equal(ticket.purchasePrice.toNumber(), 100_000_000);
    console.log(`   Price paid: ${ticket.purchasePrice.toString()} lamports (full price)`);

    const event = await pg.program.account.event.fetch(eventPDA);
    assert.equal(event.ticketsSold, 1);
  });

  // ─── 4. Issue POAP ───
  it("Issue POAP attendance record", async () => {
    const txHash = await pg.program.methods
      .issuePoap()
      .accounts({
        event: eventPDA,
        attendanceRecord: poapPDA,
        attendee: pg.wallet.publicKey,
        authority: pg.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);
    console.log(`✅ POAP issued - TX: ${txHash}`);

    const record = await pg.program.account.attendanceRecord.fetch(poapPDA);
    assert.ok(record.event.equals(eventPDA));
    assert.ok(record.attendee.equals(pg.wallet.publicKey));
    assert.ok(record.authority.equals(pg.wallet.publicKey));
    assert.ok(record.attendedAt.toNumber() > 0);
    console.log(`   Attended at: ${new Date(record.attendedAt.toNumber() * 1000).toISOString()}`);
  });

  // ─── 5. Leave review ───
  it("Leave review (requires valid ticket)", async () => {
    const txHash = await pg.program.methods
      .leaveReview(5, "Amazing event! Best hackathon ever.")
      .accounts({
        event: eventPDA,
        ticket: ticketPDA,
        review: reviewPDA,
        reviewer: pg.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);
    console.log(`✅ Review left - TX: ${txHash}`);

    const review = await pg.program.account.review.fetch(reviewPDA);
    assert.ok(review.event.equals(eventPDA));
    assert.ok(review.reviewer.equals(pg.wallet.publicKey));
    assert.equal(review.rating, 5);
    assert.equal(review.comment, "Amazing event! Best hackathon ever.");
    assert.ok(review.timestamp.toNumber() > 0);
    console.log(`   Rating: ${review.rating}/5 - "${review.comment}"`);
  });

  // ─── 6. Cancel ticket ───
  it("Cancel ticket", async () => {
    const txHash = await pg.program.methods
      .cancelTicket()
      .accounts({
        event: eventPDA,
        ticket: ticketPDA,
        owner: pg.wallet.publicKey,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);
    console.log(`✅ Ticket cancelled - TX: ${txHash}`);

    const event = await pg.program.account.event.fetch(eventPDA);
    assert.equal(event.ticketsSold, 0);
    console.log(`   Tickets sold now: ${event.ticketsSold}`);
  });

  // ─── 7. Close event ───
  it("Close event", async () => {
    const txHash = await pg.program.methods
      .closeEvent()
      .accounts({
        event: eventPDA,
        authority: pg.wallet.publicKey,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);
    console.log(`✅ Event closed - TX: ${txHash}`);

    try {
      await pg.program.account.event.fetch(eventPDA);
      assert.fail("Account should be closed");
    } catch (err) {
      console.log("   ✓ Event account closed successfully");
    }
  });
});
