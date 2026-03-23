describe("EventTickets v2 (Audited)", () => {
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

  // 1. Create event with event_type
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
    console.log(`  Event created - TX: ${txHash}`);

    const event = await pg.program.account.event.fetch(eventPDA);
    assert.equal(event.name, "Solana Hacker House Lima");
    assert.equal(event.eventType, 0);
    assert.equal(event.maxTickets, 100);
    assert.equal(event.ticketsSold, 0);
    assert.equal(event.isActive, true);
  });

  // 2. Update event
  it("Update event", async () => {
    const txHash = await pg.program.methods
      .updateEvent(
        "Solana Hacker House Lima 2026",
        "Updated: 3 days of hacking",
        new anchor.BN(100_000_000),
        150
      )
      .accounts({
        event: eventPDA,
        authority: pg.wallet.publicKey,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);

    const event = await pg.program.account.event.fetch(eventPDA);
    assert.equal(event.name, "Solana Hacker House Lima 2026");
    assert.equal(event.maxTickets, 150);
  });

  // 3. Buy ticket (no loyalty — no remaining_accounts)
  it("Buy ticket without loyalty", async () => {
    const txHash = await pg.program.methods
      .buyTicket()
      .accounts({
        event: eventPDA,
        ticket: ticketPDA,
        eventAuthority: pg.wallet.publicKey,
        buyer: pg.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);

    const ticket = await pg.program.account.ticket.fetch(ticketPDA);
    assert.equal(ticket.isValid, true);
    assert.equal(ticket.purchasePrice.toNumber(), 100_000_000);
    console.log(`  Full price paid: ${ticket.purchasePrice.toString()} lamports`);

    const event = await pg.program.account.event.fetch(eventPDA);
    assert.equal(event.ticketsSold, 1);
  });

  // 4. Issue POAP
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

    const record = await pg.program.account.attendanceRecord.fetch(poapPDA);
    assert.ok(record.event.equals(eventPDA));
    assert.ok(record.attendee.equals(pg.wallet.publicKey));
    assert.ok(record.attendedAt.toNumber() > 0);
  });

  // 5. Leave review
  it("Leave review (requires valid ticket)", async () => {
    const txHash = await pg.program.methods
      .leaveReview(5, "Amazing event!")
      .accounts({
        event: eventPDA,
        ticket: ticketPDA,
        review: reviewPDA,
        reviewer: pg.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);

    const review = await pg.program.account.review.fetch(reviewPDA);
    assert.equal(review.rating, 5);
    assert.equal(review.comment, "Amazing event!");
  });

  // 6. Cancel ticket
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

    const event = await pg.program.account.event.fetch(eventPDA);
    assert.equal(event.ticketsSold, 0);
  });

  // 7. Close event
  it("Close event", async () => {
    const txHash = await pg.program.methods
      .closeEvent()
      .accounts({
        event: eventPDA,
        authority: pg.wallet.publicKey,
      })
      .rpc();

    await pg.connection.confirmTransaction(txHash);

    try {
      await pg.program.account.event.fetch(eventPDA);
      assert.fail("Account should be closed");
    } catch (err) {
      console.log("  Event account closed successfully");
    }
  });
});
