#![no_std]
#![no_main]

use panic_halt as _;
use rtic::app;

mod smbus;

#[app(device = stm32f0xx_hal::pac, peripherals = true)]
mod app {
    use rtt_target::{rprintln, rtt_init_print};

    use crate::smbus::Data;
    use stm32f0xx_hal::gpio::gpiob::PB8;
    use stm32f0xx_hal::gpio::gpiob::PB9;
    use stm32f0xx_hal::gpio::Alternate;
    use stm32f0xx_hal::gpio::AF1;

    use stm32f0xx_hal::{
        gpio::gpioa::PA5,
        gpio::gpioc::PC13,
        gpio::{Floating, Input, Output, PushPull},
        pac,
        prelude::*,
    };

    use cortex_m::interrupt::free as disable_interrupts;

    use smbus_request_parser::*;
    #[shared]
    struct Shared {
        exti: pac::EXTI,
        user_button: PC13<Input<Floating>>,
        led: PA5<Output<PushPull>>,
        handler: Data,
    }

    #[local]
    struct Local {
        bus_state: SMBusState,
        i2c: pac::I2C1,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        let mut dp: pac::Peripherals = ctx.device;

        dp.RCC.apb2enr.modify(|_, w| w.syscfgen().set_bit());
        dp.RCC
            .apb1enr
            .modify(|_, w| w.i2c1en().set_bit().pwren().set_bit());
        dp.RCC.cfgr3.modify(|_, w| w.i2c1sw().sysclk());
        dp.RCC.ahbenr.modify(|_, w| w.iopben().set_bit());

        let mut rcc = dp
            .RCC
            .configure()
            .usbsrc(stm32f0xx_hal::rcc::USBClockSource::HSI48)
            .hsi48()
            .enable_crs(dp.CRS)
            .sysclk(48.mhz())
            .pclk(24.mhz())
            .freeze(&mut dp.FLASH);

        let gpioa = dp.GPIOA.split(&mut rcc);
        let led = disable_interrupts(|cs| gpioa.pa5.into_push_pull_output(cs));

        let gpioc = dp.GPIOC.split(&mut rcc);
        let user_button = disable_interrupts(|cs| gpioc.pc13.into_floating_input(cs));

        dp.SYSCFG.exticr4.write(|w| w.exti13().pc13());

        // Set interrupt mask for all the above
        dp.EXTI.imr.write(|w| w.mr13().set_bit());

        // Set interrupt rising trigger
        dp.EXTI.ftsr.write(|w| w.tr13().set_bit());

        let exti = dp.EXTI;

        let gpiob = dp.GPIOB.split(&mut rcc);

        let (_scl, _sda): (PB8<Alternate<AF1>>, PB9<Alternate<AF1>>) = disable_interrupts(|cs| {
            (
                gpiob.pb8.into_alternate_af1(cs),
                gpiob.pb9.into_alternate_af1(cs),
            )
        });

        const I2C_MODE_SMBUS_AUTOEND_WITH_PEC: u32 = (1 << 25) | (1 << 26);

        unsafe {
            dp.I2C1.icr.reset();

            dp.I2C1.timingr.write(|w| w.bits(0x0000020B));
            dp.I2C1.cr2.modify(|_, w| w.autoend().set_bit());
            dp.I2C1.oar1.modify(|_, w| w.oa1en().clear_bit());
            dp.I2C1.oar2.modify(|_, w| w.oa2en().clear_bit());
            dp.I2C1.cr1.modify(|_, w| w.gcen().clear_bit());
            dp.I2C1.cr1.modify(|_, w| w.nostretch().clear_bit());
            dp.I2C1.cr1.modify(|_, w| w.pe().clear_bit());
            dp.I2C1
                .cr1
                .modify(|_, w| w.anfoff().set_bit().dnf().no_filter());
            dp.I2C1.cr1.modify(|_, w| w.pe().set_bit());
            dp.I2C1.oar1.write(|w| w.oa1().bits(0x32));
            dp.I2C1
                .cr1
                .modify(|r, w| w.bits(r.bits() | I2C_MODE_SMBUS_AUTOEND_WITH_PEC));
            //dp.I2C1.cr2.modify(|_, w| w.nack().set_bit());
            //dp.I2C1.cr1.modify(|_, w| w.pecen().set_bit());
            dp.I2C1.oar1.modify(|_, w| w.oa1en().set_bit());
            dp.I2C1.cr1.modify(|_, w| w.sbc().clear_bit());

            dp.I2C1.cr1.modify(|_, w| w.txie().set_bit());
            dp.I2C1.cr1.modify(|_, w| w.rxie().set_bit());
            dp.I2C1.cr1.modify(|_, w| w.addrie().set_bit());
            dp.I2C1.cr1.modify(|_, w| w.nackie().set_bit());
            dp.I2C1.cr1.modify(|_, w| w.stopie().set_bit());
            dp.I2C1.cr1.modify(|_, w| w.tcie().set_bit());
            dp.I2C1.cr1.modify(|_, w| w.errie().set_bit());
            dp.I2C1.cr2.modify(|_, w| w.nbytes().bits(0x1));
        }

        let bus_state = SMBusState::default();
        let handler = Data::default();

        let shared = Shared {
            exti,
            user_button,
            led,
            handler,
        };
        let local = Local {
            i2c: dp.I2C1,
            bus_state,
        };
        (shared, local, init::Monotonics())
    }

    #[idle()]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
            // disable wfe for now (no probe-rs)
            //cortex_m::asm::wfe();
        }
    }

    #[task(binds = I2C1, local = [i2c, bus_state], shared = [user_button, led, handler], priority = 1)]
    fn i2c1_interrupt(mut ctx: i2c1_interrupt::Context) {
        let isr_reader = ctx.local.i2c.isr.read();

        if isr_reader.addr().is_match() {
            /* Clear address match interrupt flag */
            ctx.local.i2c.icr.write(|w| w.addrcf().set_bit());
            if isr_reader.dir().is_read() {
                /* Set TXE in ISR (not exposed by svd, so unsafe) */
                ctx.local
                    .i2c
                    .isr
                    .modify(|r, w| unsafe { w.bits(r.bits() | 1) });

                ctx.local.i2c.cr1.modify(|_, w| w.txie().set_bit());
                let mut address_match_event = I2cEvent::Initiated {
                    direction: Direction::SlaveToMaster,
                };
                if let Err(protocol_error) = ctx.shared.handler.lock(|handler| {
                    handler.handle_i2c_event(&mut address_match_event, ctx.local.bus_state)
                }) {
                    rprintln!("{:?}", protocol_error);
                }
                rprintln!("{:?}", address_match_event);
            } else {
                let mut address_match_event = I2cEvent::Initiated {
                    direction: Direction::MasterToSlave,
                };
                if let Err(protocol_error) = ctx.shared.handler.lock(|handler| {
                    handler.handle_i2c_event(&mut address_match_event, ctx.local.bus_state)
                }) {
                    rprintln!("{:?}", protocol_error);
                }
                rprintln!("{:?}", address_match_event);
            }
        } else if isr_reader.txis().is_empty() {
            ctx.local.i2c.cr1.modify(|_, w| w.txie().clear_bit());
            let mut byte: u8 = 0;
            let mut txis_event = I2cEvent::RequestedByte { byte: &mut byte };

            if let Err(protocol_error) = ctx
                .shared
                .handler
                .lock(|handler| handler.handle_i2c_event(&mut txis_event, ctx.local.bus_state))
            {
                rprintln!("{:?}", protocol_error);
            }
            rprintln!("{:x?}", txis_event);

            /* Set the transmit register */
            // does this also clear the interrupt flag?
            ctx.local.i2c.txdr.write(|w| w.txdata().bits(byte));
        }

        /* Handle receive buffer not empty */
        if isr_reader.rxne().is_not_empty() {
            let data = ctx.local.i2c.rxdr.read().rxdata().bits();

            let mut rxne_event = I2cEvent::ReceivedByte { byte: data };
            rprintln!("{:x?}", rxne_event);

            if let Err(protocol_error) = ctx
                .shared
                .handler
                .lock(|handler| handler.handle_i2c_event(&mut rxne_event, ctx.local.bus_state))
            {
                rprintln!("{:?}", protocol_error);
            }
        }

        /* Handle Stop */
        if isr_reader.stopf().is_stop() {
            ctx.local.i2c.icr.write(|w| w.stopcf().set_bit());

            let mut stop_event = I2cEvent::Stopped;
            rprintln!("{:?}", stop_event);

            if let Err(protocol_error) = ctx
                .shared
                .handler
                .lock(|handler| handler.handle_i2c_event(&mut stop_event, ctx.local.bus_state))
            {
                rprintln!("{:?}", protocol_error);
            }
        }
        /* TODO Read error flags */
    }

    #[task(binds = EXTI4_15, shared = [exti, user_button, led])]
    fn exti_4_15_interrupt(mut ctx: exti_4_15_interrupt::Context) {
        match ctx.shared.exti.lock(|exti| exti.pr.read().bits()) {
            0x2000 => {
                ctx.shared
                    .exti
                    .lock(|exti| exti.pr.write(|w| w.pif13().set_bit()));
                if ctx
                    .shared
                    .user_button
                    .lock(|user_button| user_button.is_high().unwrap_or_default())
                {
                    ctx.shared.led.lock(|led| led.set_high().unwrap());
                    rprintln!("PC13 triggered: high");
                } else {
                    ctx.shared.led.lock(|led| led.set_low().unwrap());
                    rprintln!("PC13 triggered: low");
                }
            }
            x => rprintln!("{}: Some other bits were pushed around on EXTI4_15 ;)", x),
        }
    }
}
