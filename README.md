# 🎟️ EventTickets - Sistema de Gestión de Eventos en Solana

CRUD completo de eventos y tickets on-chain desarrollado con **Rust + Anchor** para la blockchain de Solana.

> 📚 Proyecto final para el **Solana LATAM Builders Program** de WayLearn × Solana Foundation.

> **Program ID:** `EFzK4HY7f8yr9qqsMJcPunCTwHF9cA69h265UR58bvj1`

---

## 🚀 Operaciones del Programa

| Instrucción      | Tipo   | Descripción                                      |
| ---------------- | ------ | ------------------------------------------------ |
| `create_event`   | CREATE | Crea un nuevo evento con nombre, precio y capacidad |
| `update_event`   | UPDATE | Actualiza los datos de un evento (solo el creador)  |
| `buy_ticket`     | CREATE | Compra un ticket para un evento (crea PDA de ticket)|
| `cancel_ticket`  | DELETE | Cancela un ticket y cierra la cuenta               |
| `close_event`    | DELETE | Cierra un evento y recupera el rent (solo el creador)|

---

## 📐 PDAs (Program Derived Addresses)

- **Event PDA:** `[authority, "event", event_id]`
- **Ticket PDA:** `[event_pda, "ticket", buyer]`

---

## 📦 Estructura de Datos

### Event Account (320 bytes)

| Campo          | Tipo     | Tamaño  | Descripción                        |
| -------------- | -------- | ------- | ---------------------------------- |
| discriminator  | u8[8]    | 8 bytes | Identificador de Anchor            |
| authority      | Pubkey   | 32 bytes| Wallet del creador del evento      |
| event_id       | u64      | 8 bytes | ID único del evento                |
| name           | String   | 4 + 50  | Nombre del evento (máx 50 chars)   |
| description    | String   | 4 + 200 | Descripción (máx 200 chars)        |
| ticket_price   | u64      | 8 bytes | Precio del ticket en lamports      |
| max_tickets    | u16      | 2 bytes | Capacidad máxima de tickets        |
| tickets_sold   | u16      | 2 bytes | Tickets vendidos                   |
| is_active      | bool     | 1 byte  | Estado del evento                  |
| bump           | u8       | 1 byte  | Bump de la PDA                     |

### Ticket Account (82 bytes)

| Campo          | Tipo     | Tamaño  | Descripción                        |
| -------------- | -------- | ------- | ---------------------------------- |
| discriminator  | u8[8]    | 8 bytes | Identificador de Anchor            |
| event          | Pubkey   | 32 bytes| PDA del evento asociado            |
| owner          | Pubkey   | 32 bytes| Wallet del comprador               |
| purchase_price | u64      | 8 bytes | Precio pagado en lamports          |
| is_valid       | bool     | 1 byte  | Validez del ticket                 |
| bump           | u8       | 1 byte  | Bump de la PDA                     |

---

## 🔒 Seguridad

- **`has_one` constraints:** Valida que solo el creador pueda modificar/cerrar sus eventos
- **Validaciones de entrada:** Límites de longitud en nombre (50) y descripción (200), capacidad > 0
- **PDA seeds:** Derivación determinista que previene colisiones y accesos no autorizados
- **Transferencia segura de SOL:** Usa `system_instruction::transfer` con `invoke` (no manipulación directa de lamports)
- **Errores personalizados:** 8 errores específicos con mensajes descriptivos en español
- **`close` constraint:** Recuperación segura de rent al cancelar tickets o cerrar eventos

---

## 🛠️ Uso con Solana Playground

1. Abre [Solana Playground](https://beta.solpg.io)
2. Crea un nuevo proyecto e importa los archivos de `src/`, `client/` y `tests/`
3. Conecta tu wallet de Devnet (menú lateral izquierdo)
4. **Build** el programa (icono de martillo o `Build`)
5. **Deploy** a Devnet (icono de flecha o `Deploy`)
6. **Test** ejecuta los 5 tests del CRUD (icono de tubo de ensayo o `Test`)
7. **Client** ejecuta el script de demostración (`client/client.ts`)

---

## 📁 Estructura del Proyecto

```
EventTickets-Solana/
├── src/
│   └── lib.rs              # Programa Anchor (CRUD completo)
├── client/
│   └── client.ts            # Script de demostración
├── tests/
│   └── event_tickets.test.ts # Tests del CRUD (5 tests)
└── README.md
```

---

## 🧰 Tecnologías

- **Rust** — Lenguaje del programa on-chain
- **Anchor** — Framework para desarrollo en Solana
- **Solana Playground** — IDE web para build, deploy y test
- **Devnet** — Red de pruebas de Solana

---

## 👤 Autor

**Roberto** — Full Stack Developer & Web3 Builder

---

## 📄 Licencia

MIT
