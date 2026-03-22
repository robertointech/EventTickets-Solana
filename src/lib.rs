use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use mpl_token_metadata::{
    instructions::CreateV1CpiBuilder,
    types::{PrintSupply, TokenStandard},
};

declare_id!("EFzK4HY7f8yr9qqsMJcPunCTwHF9cA69h265UR58bvj1");

/// Event type constants
pub const EVENT_TYPE_CONFERENCE: u8 = 0;
pub const EVENT_TYPE_RESEARCH: u8 = 1;
pub const EVENT_TYPE_ART: u8 = 2;
pub const EVENT_TYPE_COMMUNITY: u8 = 3;
pub const EVENT_TYPE_OTHER: u8 = 4;

/// Loyalty threshold: 3+ POAPs from same authority = 20% discount
pub const LOYALTY_THRESHOLD: u8 = 3;
pub const LOYALTY_DISCOUNT_BPS: u64 = 2000; // 20% in basis points

#[program]
pub mod event_tickets {
    use super::*;

    /// CREATE - Crear un nuevo evento con tipo
    pub fn create_event(
        ctx: Context<CreateEvent>,
        event_id: u64,
        name: String,
        description: String,
        ticket_price: u64,
        max_tickets: u16,
        event_type: u8,
    ) -> Result<()> {
        require!(name.len() <= 50, EventError::NameTooLong);
        require!(description.len() <= 200, EventError::DescriptionTooLong);
        require!(max_tickets > 0, EventError::InvalidCapacity);
        require!(event_type <= EVENT_TYPE_OTHER, EventError::InvalidEventType);

        let event = &mut ctx.accounts.event;
        event.authority = ctx.accounts.authority.key();
        event.event_id = event_id;
        event.name = name;
        event.description = description;
        event.ticket_price = ticket_price;
        event.max_tickets = max_tickets;
        event.tickets_sold = 0;
        event.is_active = true;
        event.event_type = event_type;
        event.bump = ctx.bumps.event;

        msg!("Event created: {} (ID: {}, type: {})", event.name, event.event_id, event.event_type);
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

        msg!("Event updated: {}", event.name);
        Ok(())
    }

    /// BUY TICKET - Comprar ticket con descuento por loyalty
    /// Si el comprador tiene 3+ AttendanceRecords del mismo authority, 20% off
    pub fn buy_ticket(ctx: Context<BuyTicket>, loyalty_count: u8) -> Result<()> {
        let event = &mut ctx.accounts.event;

        require!(event.is_active, EventError::EventNotActive);
        require!(
            event.tickets_sold < event.max_tickets,
            EventError::EventSoldOut
        );

        // Calculate price with potential loyalty discount
        let mut final_price = event.ticket_price;
        if loyalty_count >= LOYALTY_THRESHOLD {
            // Loyalty discount: 20% off
            let discount = event
                .ticket_price
                .checked_mul(LOYALTY_DISCOUNT_BPS)
                .unwrap()
                .checked_div(10_000)
                .unwrap();
            final_price = event.ticket_price.checked_sub(discount).unwrap();
            msg!("Loyalty discount applied! {} -> {} lamports", event.ticket_price, final_price);
        }

        if final_price > 0 {
            let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.buyer.key(),
                &ctx.accounts.event_authority.key(),
                final_price,
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
        ticket.purchase_price = final_price;
        ticket.is_valid = true;
        ticket.bump = ctx.bumps.ticket;

        event.tickets_sold += 1;

        msg!(
            "Ticket purchased for '{}' by {} (paid: {} lamports)",
            event.name,
            ctx.accounts.buyer.key(),
            final_price
        );
        Ok(())
    }

    /// MINT TICKET NFT - Mint an NFT representing the ticket
    /// Uses Metaplex Token Metadata via CPI
    pub fn mint_ticket_nft(ctx: Context<MintTicketNft>, uri: String) -> Result<()> {
        let event = &ctx.accounts.event;
        let ticket = &ctx.accounts.ticket;

        require!(ticket.is_valid, EventError::TicketNotValid);
        require!(ticket.owner == ctx.accounts.buyer.key(), EventError::NotTicketOwner);

        // Mint one token to the buyer's token account
        let cpi_accounts = MintTo {
            mint: ctx.accounts.nft_mint.to_account_info(),
            to: ctx.accounts.nft_token_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::mint_to(cpi_ctx, 1)?;

        // Create metadata via Metaplex CPI
        let nft_name = format!("{} - Ticket #{}", event.name, event.tickets_sold);
        // Truncate to 32 chars (Metaplex limit)
        let nft_name = if nft_name.len() > 32 {
            nft_name[..32].to_string()
        } else {
            nft_name
        };

        CreateV1CpiBuilder::new(&ctx.accounts.token_metadata_program)
            .metadata(&ctx.accounts.metadata)
            .mint(&ctx.accounts.nft_mint.to_account_info(), false)
            .authority(&ctx.accounts.buyer.to_account_info())
            .payer(&ctx.accounts.buyer.to_account_info())
            .update_authority(&ctx.accounts.buyer.to_account_info(), true)
            .system_program(&ctx.accounts.system_program.to_account_info())
            .sysvar_instructions(&ctx.accounts.sysvar_instructions)
            .spl_token_program(&ctx.accounts.token_program.to_account_info())
            .name(nft_name)
            .symbol(String::from("BTKT"))
            .uri(uri)
            .seller_fee_basis_points(0)
            .token_standard(TokenStandard::NonFungible)
            .print_supply(PrintSupply::Zero)
            .invoke()?;

        msg!("NFT ticket minted for event '{}'", event.name);
        Ok(())
    }

    /// ISSUE POAP - Authority issues attendance record
    pub fn issue_poap(ctx: Context<IssuePoap>) -> Result<()> {
        let event = &ctx.accounts.event;
        let clock = Clock::get()?;

        let record = &mut ctx.accounts.attendance_record;
        record.event = event.key();
        record.attendee = ctx.accounts.attendee.key();
        record.authority = event.authority;
        record.attended_at = clock.unix_timestamp;
        record.bump = ctx.bumps.attendance_record;

        msg!(
            "POAP issued for '{}' to {}",
            event.name,
            ctx.accounts.attendee.key()
        );
        Ok(())
    }

    /// LEAVE REVIEW - Ticket holder leaves a review
    pub fn leave_review(ctx: Context<LeaveReview>, rating: u8, comment: String) -> Result<()> {
        require!(rating >= 1 && rating <= 5, EventError::InvalidRating);
        require!(comment.len() <= 280, EventError::CommentTooLong);

        let ticket = &ctx.accounts.ticket;
        require!(ticket.is_valid, EventError::TicketNotValid);

        let clock = Clock::get()?;
        let review = &mut ctx.accounts.review;
        review.event = ctx.accounts.event.key();
        review.reviewer = ctx.accounts.reviewer.key();
        review.rating = rating;
        review.comment = comment;
        review.timestamp = clock.unix_timestamp;
        review.bump = ctx.bumps.review;

        msg!(
            "Review left for '{}' by {} (rating: {})",
            ctx.accounts.event.name,
            ctx.accounts.reviewer.key(),
            rating
        );
        Ok(())
    }

    /// DELETE (Ticket) - Cancelar un ticket
    pub fn cancel_ticket(ctx: Context<CancelTicket>) -> Result<()> {
        let event = &mut ctx.accounts.event;
        event.tickets_sold -= 1;

        msg!(
            "Ticket cancelled for '{}' by {}",
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

        msg!("Event closed: {} (ID: {})", event.name, event.event_id);
        Ok(())
    }
}

// ═══════════════════════════════════════════════
// CONTEXTOS
// ═══════════════════════════════════════════════

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
pub struct MintTicketNft<'info> {
    #[account(
        seeds = [event.authority.as_ref(), b"event", &event.event_id.to_le_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Event>,

    #[account(
        has_one = owner @ EventError::NotTicketOwner,
        seeds = [event.key().as_ref(), b"ticket", buyer.key().as_ref()],
        bump = ticket.bump
    )]
    pub ticket: Account<'info, Ticket>,

    #[account(mut)]
    pub nft_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = nft_token_account.mint == nft_mint.key(),
        constraint = nft_token_account.owner == buyer.key()
    )]
    pub nft_token_account: Account<'info, TokenAccount>,

    /// CHECK: Metaplex metadata PDA, validated by token metadata program
    #[account(mut)]
    pub metadata: AccountInfo<'info>,

    #[account(
        mut,
        constraint = buyer.key() == ticket.owner @ EventError::NotTicketOwner
    )]
    pub buyer: Signer<'info>,

    /// CHECK: Token Metadata program
    #[account(address = mpl_token_metadata::ID)]
    pub token_metadata_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    /// CHECK: Sysvar instructions
    #[account(address = sysvar::instructions::id())]
    pub sysvar_instructions: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct IssuePoap<'info> {
    #[account(
        has_one = authority,
        seeds = [authority.key().as_ref(), b"event", &event.event_id.to_le_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Event>,

    #[account(
        init,
        payer = authority,
        space = AttendanceRecord::SPACE,
        seeds = [attendee.key().as_ref(), b"poap", event.key().as_ref()],
        bump
    )]
    pub attendance_record: Account<'info, AttendanceRecord>,

    /// CHECK: The attendee wallet receiving the POAP
    pub attendee: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LeaveReview<'info> {
    #[account(
        seeds = [event.authority.as_ref(), b"event", &event.event_id.to_le_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Event>,

    #[account(
        has_one = owner @ EventError::NotTicketOwner,
        seeds = [event.key().as_ref(), b"ticket", reviewer.key().as_ref()],
        bump = ticket.bump,
        constraint = ticket.owner == reviewer.key() @ EventError::NotTicketOwner
    )]
    pub ticket: Account<'info, Ticket>,

    #[account(
        init,
        payer = reviewer,
        space = Review::SPACE,
        seeds = [event.key().as_ref(), b"review", reviewer.key().as_ref()],
        bump
    )]
    pub review: Account<'info, Review>,

    #[account(mut)]
    pub reviewer: Signer<'info>,

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

// ═══════════════════════════════════════════════
// CUENTAS
// ═══════════════════════════════════════════════

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
    pub event_type: u8,
    pub bump: u8,
}

impl Event {
    // 8 (discriminator) + 32 + 8 + (4+50) + (4+200) + 8 + 2 + 2 + 1 + 1 + 1
    pub const SPACE: usize = 8 + 32 + 8 + (4 + 50) + (4 + 200) + 8 + 2 + 2 + 1 + 1 + 1;
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

#[account]
pub struct AttendanceRecord {
    pub event: Pubkey,
    pub attendee: Pubkey,
    pub authority: Pubkey,
    pub attended_at: i64,
    pub bump: u8,
}

impl AttendanceRecord {
    // 8 + 32 + 32 + 32 + 8 + 1
    pub const SPACE: usize = 8 + 32 + 32 + 32 + 8 + 1;
}

#[account]
pub struct Review {
    pub event: Pubkey,
    pub reviewer: Pubkey,
    pub rating: u8,
    pub comment: String,
    pub timestamp: i64,
    pub bump: u8,
}

impl Review {
    // 8 + 32 + 32 + 1 + (4+280) + 8 + 1
    pub const SPACE: usize = 8 + 32 + 32 + 1 + (4 + 280) + 8 + 1;
}

// ═══════════════════════════════════════════════
// ERRORES
// ═══════════════════════════════════════════════

#[error_code]
pub enum EventError {
    #[msg("Event name cannot exceed 50 characters.")]
    NameTooLong,
    #[msg("Description cannot exceed 200 characters.")]
    DescriptionTooLong,
    #[msg("Capacity must be greater than 0.")]
    InvalidCapacity,
    #[msg("Event is not active.")]
    EventNotActive,
    #[msg("Event is sold out.")]
    EventSoldOut,
    #[msg("Cannot reduce capacity below tickets already sold.")]
    CannotReduceBelowSold,
    #[msg("Event still has tickets sold, cannot close.")]
    EventHasTickets,
    #[msg("Event authority mismatch.")]
    InvalidAuthority,
    #[msg("Invalid event type (must be 0-4).")]
    InvalidEventType,
    #[msg("Ticket is not valid.")]
    TicketNotValid,
    #[msg("Not the ticket owner.")]
    NotTicketOwner,
    #[msg("Rating must be between 1 and 5.")]
    InvalidRating,
    #[msg("Comment cannot exceed 280 characters.")]
    CommentTooLong,
}
