describe("EventTickets", () => {
  const eventId = new anchor.BN(1);

  const [eventPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      pg.wallet.publicKey.toBuffer(),
      Buffer.from("event"),
      eventId.toArrayLike(Buffer, "le", 8),
    ],
    pg.PROGRAM_ID
  );

  it("Crear un evento", async () => {
    const txHash = await pg.program.methods
      .createEvent(
        eventId,
        "Ethereum Lima Day 2025",
        "Evento de blockchain en Lima",
        new anchor.BN(100000000),
        100
      )
      .accounts({
        event: eventPDA,
        authority: pg.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log(`✅ Evento creado - TX: ${txHash}`);
    await pg.connection.confirmTransaction(txHash);

    const eventAccount = await pg.program.account.event.fetch(eventPDA);

    console.log("📋 Datos del evento:");
    console.log(`   Nombre: ${eventAccount.name}`);
    console.log(`   Descripción: ${eventAccount.description}`);
    console.log(`   Precio: ${eventAccount.ticketPrice.toString()} lamports`);
    console.log(`   Capacidad: ${eventAccount.maxTickets}`);
    console.log(`   Tickets vendidos: ${eventAccount.ticketsSold}`);
    console.log(`   Activo: ${eventAccount.isActive}`);

    assert.equal(eventAccount.name, "Ethereum Lima Day 2025");
    assert.equal(eventAccount.maxTickets, 100);
    assert.equal(eventAccount.ticketsSold, 0);
    assert.equal(eventAccount.isActive, true);
  });

  it("Actualizar un evento", async () => {
    const txHash = await pg.program.methods
      .updateEvent(
        "Solana Lima Day 2025",
        "Evento actualizado de Solana en Lima",
        new anchor.BN(200000000),
        150
      )
      .accounts({
        event: eventPDA,
        authority: pg.wallet.publicKey,
      })
      .rpc();

    console.log(`✅ Evento actualizado - TX: ${txHash}`);
    await pg.connection.confirmTransaction(txHash);

    const eventAccount = await pg.program.account.event.fetch(eventPDA);
    console.log(`   Nuevo nombre: ${eventAccount.name}`);
    console.log(`   Nuevo precio: ${eventAccount.ticketPrice.toString()} lamports`);

    assert.equal(eventAccount.name, "Solana Lima Day 2025");
    assert.equal(eventAccount.maxTickets, 150);
  });

  it("Comprar un ticket", async () => {
    const [ticketPDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        eventPDA.toBuffer(),
        Buffer.from("ticket"),
        pg.wallet.publicKey.toBuffer(),
      ],
      pg.PROGRAM_ID
    );

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

    console.log(`✅ Ticket comprado - TX: ${txHash}`);
    await pg.connection.confirmTransaction(txHash);

    const ticketAccount = await pg.program.account.ticket.fetch(ticketPDA);
    console.log(`🎟️  Ticket válido: ${ticketAccount.isValid}`);
    console.log(`   Precio pagado: ${ticketAccount.purchasePrice.toString()} lamports`);

    assert.equal(ticketAccount.isValid, true);

    const eventAccount = await pg.program.account.event.fetch(eventPDA);
    console.log(`   Tickets vendidos ahora: ${eventAccount.ticketsSold}`);
    assert.equal(eventAccount.ticketsSold, 1);
  });

  it("Cancelar un ticket", async () => {
    const [ticketPDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        eventPDA.toBuffer(),
        Buffer.from("ticket"),
        pg.wallet.publicKey.toBuffer(),
      ],
      pg.PROGRAM_ID
    );

    const txHash = await pg.program.methods
      .cancelTicket()
      .accounts({
        event: eventPDA,
        ticket: ticketPDA,
        owner: pg.wallet.publicKey,
      })
      .rpc();

    console.log(`✅ Ticket cancelado - TX: ${txHash}`);
    await pg.connection.confirmTransaction(txHash);

    const eventAccount = await pg.program.account.event.fetch(eventPDA);
    console.log(`   Tickets vendidos ahora: ${eventAccount.ticketsSold}`);
    assert.equal(eventAccount.ticketsSold, 0);
  });

  it("Cerrar un evento", async () => {
    const txHash = await pg.program.methods
      .closeEvent()
      .accounts({
        event: eventPDA,
        authority: pg.wallet.publicKey,
      })
      .rpc();

    console.log(`✅ Evento cerrado - TX: ${txHash}`);
    await pg.connection.confirmTransaction(txHash);

    try {
      await pg.program.account.event.fetch(eventPDA);
      assert.fail("La cuenta debería estar cerrada");
    } catch (err) {
      console.log("   ✓ Cuenta del evento cerrada correctamente");
    }
  });
});
