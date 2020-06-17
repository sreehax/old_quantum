use crate::{console, cpu, driver, synchronization, synchronization::NullLock};
use core::{fmt, ops};
use register::{mmio::*, register_bitfields, register_structs};

register_bitfields! {
    u32,

    // Flag Register
    FR [
        // Transmit FIFO empty
        TXFE OFFSET(7) NUMBITS(1) [],
        // Transmit FIFO full
        TXFF OFFSET(5) NUMBITS(1) [],
        // Receive FIFO empty
        RXFE OFFSET(4) NUMBITS(1) []
    ],
    // Integer Baud rate divisor
    IBRD [
        IBRD OFFSET(0) NUMBITS(16) []
    ],

    // Fractional Baud rate divisor
    FBRD [
        FBRD OFFSET(0) NUMBITS(6) []
    ],
    // Line Control register
    LCRH [
        // Word length
        WLEN OFFSET(5) NUMBITS(2) [
            FiveBit = 0b00,
            SixBit = 0b01,
            SevenBit = 0b10,
            EightBit = 0b11
        ],
        // Enable FIFOs
        FEN  OFFSET(4) NUMBITS(1) [
            FifosDisabled = 0,
            FifosEnabled = 1
        ]
    ],

    // Control Register
    CR [
        // Receive enable
        RXE    OFFSET(9) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1
        ],
        // Transmit enable
        TXE    OFFSET(8) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1
        ],
        // UART enable
        UARTEN OFFSET(0) NUMBITS(1) [
           Disabled = 0,
            Enabled = 1
        ]
    ],
    // Interrupt Clear Register
    ICR [
        // Meta field for all pending interrupts
        ALL OFFSET(0) NUMBITS(11) []
    ]
}


register_structs! {
    #[allow(non_snake_case)]
    pub RegisterBlock {
        (0x00 => DR: ReadWrite<u32>),
        (0x04 => _reserved1),
        (0x18 => FR: ReadOnly<u32, FR::Register>),
        (0x1c => _reserved2),
        (0x24 => IBRD: WriteOnly<u32, IBRD::Register>),
        (0x28 => FBRD: WriteOnly<u32, FBRD::Register>),
        (0x2c => LCRH: WriteOnly<u32, LCRH::Register>),
        (0x30 => CR: WriteOnly<u32, CR::Register>),
        (0x34 => _reserved3),
        (0x44 => ICR: WriteOnly<u32, ICR::Register>),
        (0x48 => @END),
    }
}

pub struct PL011UartInner {
    base_addr: usize,
    chars_written: usize,
    chars_read: usize,
}

pub use PL011UartInner as PanicUart;

pub struct PL011Uart {
    inner: NullLock<PL011UartInner>,
}

impl ops::Deref for PL011UartInner {
    type Target = RegisterBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}

impl PL011UartInner {
    pub const unsafe fn new(base_addr: usize) -> Self {
        Self {
            base_addr,
            chars_written: 0,
            chars_read: 0,
        }
    }

    pub fn init(&mut self) {
        self.CR.set(0);

        self.ICR.write(ICR::ALL::CLEAR);
        self.IBRD.write(IBRD::IBRD.val(13));
        self.FBRD.write(FBRD::FBRD.val(2));
        self.LCRH.write(LCRH::WLEN::EightBit + LCRH::FEN::FifosEnabled);
        self.CR.write(CR::UARTEN::Enabled + CR::TXE::Enabled + CR::RXE::Enabled);
    }

    fn ptr(&self) -> *const RegisterBlock {
        self.base_addr as *const _
    }

    fn write_char(&mut self, c: char) {
        while self.FR.matches_all(FR::TXFF::SET) {
            cpu::nop();
        }
        self.DR.set(c as u32);
        self.chars_written += 1;
    }
}

impl fmt::Write for PL011UartInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }

        Ok(())
    }
}

impl PL011Uart {
    pub const unsafe fn new(base_addr: usize) -> Self {
        Self {
            inner: NullLock::new(PL011UartInner::new(base_addr)),
        }
    }
}

use synchronization::interface::Mutex;

impl driver::interface::DeviceDriver for PL011Uart {
    fn compatible(&self) -> &str {
        "BCM PL011 UART"
    }

    fn init(&self) -> Result<(), ()> {
        let mut r = &self.inner;
        r.lock(|inner| inner.init());

        Ok(())
    }
}

impl console::interface::Write for PL011Uart {
    fn write_char(&self, c: char) {
        let mut r = &self.inner;
        r.lock(|inner| inner.write_char(c));
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> fmt::Result {
        let mut r = &self.inner;
        r.lock(|inner| fmt::Write::write_fmt(inner, args))
    }
}

impl console::interface::Read for PL011Uart {
    fn read_char(&self) -> char {
        let mut r = &self.inner;
        r.lock(|inner| {
            while inner.FR.matches_all(FR::RXFE::SET) {
                cpu::nop();
            }

            let mut ret = inner.DR.get() as u8 as char;

            if ret == '\r' {
                ret = '\n';
            }

            inner.chars_read += 1;

            ret
        })
    }
}

impl console::interface::Statistics for PL011Uart {
    fn chars_written(&self) -> usize {
        let mut r = &self.inner;
        r.lock(|inner| inner.chars_written)
    }

    fn chars_read(&self) -> usize {
        let mut r = &self.inner;
        r.lock(|inner| inner.chars_read)
    }
}