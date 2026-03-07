use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

/// PROGRAMA: EventTickets
/// CRUD completo de gestión de eventos y tickets en Solana.
/// Cada evento es una PDA derivada del creador y un ID único.
/// Los tickets son PDAs derivadas del evento y el comprador.
///
/// Operaciones:
///   - create_event:  Crea un nuevo evento con nombre, descripción, precio y capacidad
///   - update_event:  Actualiza los datos de un evento existente (solo el creador)
///   - buy_ticket:    Compra un ticket para un evento (crea PDA de ticket)
///   - cancel_ticket: Cancela un ticket existente (cierra la cuenta)
///   - close_event:   Cierra un evento y recupera el rent (solo el creador)
#[program]
pub mod event_tickets {
    use super::*;

    /// CREATE - Crear un nuevo evento
    pub fn create_event(
        ctx: Context<CreateEvent>,
        event_id: u64,
        name: String,
        description: String,
        ticket_price: u64,
        max_tickets: u16,
    ) -> Result<()> {
        require!(name.len() <= 50, EventError::NameTooLong);
        require!(description.len() <= 200, EventError::DescriptionTooLong);
        require!(max_tickets > 0, EventError::InvalidCapacity);

        let event = &mut ctx.accounts.event;
        event.authority = ctx.accounts.authority.key();
        event.event_id = event_id;
        event.name = name;
        event.description = description;
        event.ticket_price = ticket_price;
        event.max_tickets = max_tickets;
        event.tickets_sold = 0;
        event.is_active = true;
        event.bump = ctx.bumps.event;

        msg!("Evento creado: {} (ID: {})", event.name, event.event_id);
        Ok(())
    }

    /// UPDATE - Actualizar datos de un evento existente
    pub fn update_event(
        ctx: Context<UpdateEvent>,
        name: String,
        description: String,
        ticket_price: u64,
        max_tickets: u16,
    ) -> Result<()> {
        let event = &mut ctx.accounts.event;

        require!(name.len() <= 50, EventError::NameTooLong);
        require!(description.len() <= 200, EventError::DescriptionTooLong);
        require!(
            max_tickets >= event.tickets_sold,
            EventError::CannotReduceBelowSold
        );

        event.name = name;
        event.description = description;
        event.ticket_price = ticket_price;
        event.max_tickets = max_tickets;

        msg!("Evento actualizado: {}", event.name);
        Ok(())
    }

    /// CREATE (Ticket) - Comprar un ticket para un evento
    pub fn buy_ticket(ctx: Context<BuyTicket>) -> Result<()> {
        let event = &mut ctx.accounts.event;

        require!(event.is_active, EventError::EventNotActive);
        require!(
            event.tickets_sold < event.max_tickets,
            EventError::EventSoldOut
        );

        if event.ticket_price > 0 {
            let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.buyer.key(),
                &ctx.accounts.event_authority.key(),
                event.ticket_price,
            );
            anchor_lang::solana_program::program::invoke(
                &transfer_ix,
                &[
                    ctx.accounts.buyer.to_account_info(),
                    ctx.accounts.event_authority.to_account_info(),
                ],
            )?;
        }

        let ticket = &mut ctx.accounts.ticket;
        ticket.event = event.key();
        ticket.owner = ctx.accounts.buyer.key();
        ticket.purchase_price = event.ticket_price;
        ticket.is_valid = true;
        ticket.bump = ctx.bumps.ticket;

        event.tickets_sold += 1;

        msg!(
            "Ticket comprado para evento '{}' por {}",
            event.name,
            ctx.accounts.buyer.key()
        );
        Ok(())
    }

    /// DELETE (Ticket) - Cancelar un ticket
    pub fn cancel_ticket(ctx: Context<CancelTicket>) -> Result<()> {
        let event = &mut ctx.accounts.event;
        event.tickets_sold -= 1;

        msg!(
            "Ticket cancelado para evento '{}' por {}",
            event.name,
            ctx.accounts.owner.key()
        );
        Ok(())
    }

    /// DELETE (Event) - Cerrar un evento
    pub fn close_event(ctx: Context<CloseEvent>) -> Result<()> {
        let event = &mut ctx.accounts.event;

        require!(
            event.tickets_sold == 0,
            EventError::EventHasTickets
        );

        msg!("Evento cerrado: {} (ID: {})", event.name, event.event_id);
        Ok(())
    }
}

// CONTEXTOS
#[derive(Accounts)]
#[instruction(event_id: u64)]
pub struct CreateEvent<'info> {
    #[account(
        init,
        payer = authority,
        space = Event::SPACE,
        seeds = [authority.key().as_ref(), b"event", &event_id.to_le_bytes()],
        bump
    )]
    pub event: Account<'info, Event>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateEvent<'info> {
    #[account(
        mut,
        has_one = authority,
        seeds = [authority.key().as_ref(), b"event", &event.event_id.to_le_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Event>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(
        mut,
        seeds = [event.authority.as_ref(), b"event", &event.event_id.to_le_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Event>,

    #[account(
        init,
        payer = buyer,
        space = Ticket::SPACE,
        seeds = [event.key().as_ref(), b"ticket", buyer.key().as_ref()],
        bump
    )]
    pub ticket: Account<'info, Ticket>,

    /// CHECK: Wallet del creador del evento que recibe el pago.
    #[account(
        mut,
        constraint = event_authority.key() == event.authority @ EventError::InvalidAuthority
    )]
    pub event_authority: AccountInfo<'info>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelTicket<'info> {
    #[account(
        mut,
        seeds = [event.authority.as_ref(), b"event", &event.event_id.to_le_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Event>,

    #[account(
        mut,
        close = owner,
        has_one = owner,
        seeds = [event.key().as_ref(), b"ticket", owner.key().as_ref()],
        bump = ticket.bump
    )]
    pub ticket: Account<'info, Ticket>,

    #[account(mut)]
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseEvent<'info> {
    #[account(
        mut,
        close = authority,
        has_one = authority,
        seeds = [authority.key().as_ref(), b"event", &event.event_id.to_le_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Event>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

// CUENTAS
#[account]
pub struct Event {
    pub authority: Pubkey,
    pub event_id: u64,
    pub name: String,
    pub description: String,
    pub ticket_price: u64,
    pub max_tickets: u16,
    pub tickets_sold: u16,
    pub is_active: bool,
    pub bump: u8,
}

impl Event {
    pub const SPACE: usize = 8 + 32 + 8 + (4 + 50) + (4 + 200) + 8 + 2 + 2 + 1 + 1;
}

#[account]
pub struct Ticket {
    pub event: Pubkey,
    pub owner: Pubkey,
    pub purchase_price: u64,
    pub is_valid: bool,
    pub bump: u8,
}

impl Ticket {
    pub const SPACE: usize = 8 + 32 + 32 + 8 + 1 + 1;
}

// ERRORES
#[error_code]
pub enum EventError {
    #[msg("El nombre del evento no puede superar los 50 caracteres.")]
    NameTooLong,
    #[msg("La descripción no puede superar los 200 caracteres.")]
    DescriptionTooLong,
    #[msg("La capacidad debe ser mayor a 0.")]
    InvalidCapacity,
    #[msg("El evento no está activo.")]
    EventNotActive,
    #[msg("El evento está agotado, no hay tickets disponibles.")]
    EventSoldOut,
    #[msg("No se puede reducir la capacidad por debajo de los tickets vendidos.")]
    CannotReduceBelowSold,
    #[msg("El evento todavía tiene tickets vendidos, no se puede cerrar.")]
    EventHasTickets,
    #[msg("La authority del evento no coincide.")]
    InvalidAuthority,
}
