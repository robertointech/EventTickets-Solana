const eventId = new anchor.BN(42);

const [eventPDA] = anchor.web3.PublicKey.findProgramAddressSync(
  [
    pg.wallet.publicKey.toBuffer(),
    Buffer.from("event"),
    eventId.toArrayLike(Buffer, "le", 8),
  ],
  pg.PROGRAM_ID
);

console.log("🔑 Program ID:", pg.PROGRAM_ID.toString());
console.log("👤 Wallet:", pg.wallet.publicKey.toString());
console.log("📍 Event PDA:", eventPDA.toString());

const txCreate = await pg.program.methods
  .createEvent(
    eventId,
    "Demo Event",
    "Evento creado desde el client script",
    new anchor.BN(50000000),
    50
  )
  .accounts({
    event: eventPDA,
    authority: pg.wallet.publicKey,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc();

console.log("\n✅ Evento creado:", txCreate);

const event = await pg.program.account.event.fetch(eventPDA);
console.log("\n📋 Evento on-chain:");
console.log("   Nombre:", event.name);
console.log("   Descripción:", event.description);
console.log("   Precio:", event.ticketPrice.toString(), "lamports");
console.log("   Capacidad:", event.maxTickets);
console.log("   Vendidos:", event.ticketsSold);
console.log("   Activo:", event.isActive);
