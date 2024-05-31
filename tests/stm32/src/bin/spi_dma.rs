#![no_std]
#![no_main]
#[path = "../common.rs"]
mod common;

use common::*;
use defmt::assert_eq;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(config());
    info!("Hello World!");

    let mut spi_peri = peri!(p, SPI);
    let mut sck = peri!(p, SPI_SCK);
    let mut mosi = peri!(p, SPI_MOSI);
    let mut miso = peri!(p, SPI_MISO);
    let mut tx_dma = peri!(p, SPI_TX_DMA);
    let mut rx_dma = peri!(p, SPI_RX_DMA);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = Hertz(1_000_000);

    let mut spi = Spi::new(
        &mut spi_peri,
        &mut sck,  // Arduino D13
        &mut mosi, // Arduino D11
        &mut miso, // Arduino D12
        &mut tx_dma,
        &mut rx_dma,
        spi_config,
    );

    let data: [u8; 9] = [0x00, 0xFF, 0xAA, 0x55, 0xC0, 0xFF, 0xEE, 0xC0, 0xDE];

    // Arduino pins D11 and D12 (MOSI-MISO) are connected together with a 1K resistor.
    // so we should get the data we sent back.
    let mut buf = [0; 9];
    spi.transfer(&mut buf, &data).await.unwrap();
    assert_eq!(buf, data);

    spi.transfer_in_place(&mut buf).await.unwrap();
    assert_eq!(buf, data);

    // Check read/write don't hang. We can't check they transfer the right data
    // without fancier test mechanisms.
    spi.write(&buf).await.unwrap();
    spi.read(&mut buf).await.unwrap();
    spi.write(&buf).await.unwrap();
    spi.read(&mut buf).await.unwrap();
    spi.write(&buf).await.unwrap();

    // Check transfer doesn't break after having done a write, due to garbage in the FIFO
    spi.transfer(&mut buf, &data).await.unwrap();
    assert_eq!(buf, data);

    // Check zero-length operations, these should be noops.
    spi.transfer::<u8>(&mut [], &[]).await.unwrap();
    spi.transfer_in_place::<u8>(&mut []).await.unwrap();
    spi.read::<u8>(&mut []).await.unwrap();
    spi.write::<u8>(&[]).await.unwrap();

    // === Check mixing blocking with async.
    spi.blocking_transfer(&mut buf, &data).unwrap();
    assert_eq!(buf, data);
    spi.transfer(&mut buf, &data).await.unwrap();
    assert_eq!(buf, data);
    spi.blocking_write(&buf).unwrap();
    spi.transfer(&mut buf, &data).await.unwrap();
    assert_eq!(buf, data);
    spi.blocking_read(&mut buf).unwrap();
    spi.blocking_write(&buf).unwrap();
    spi.write(&buf).await.unwrap();
    spi.read(&mut buf).await.unwrap();
    spi.blocking_write(&buf).unwrap();
    spi.blocking_read(&mut buf).unwrap();
    spi.write(&buf).await.unwrap();

    core::mem::drop(spi);

    // test rx-only configuration
    let mut spi = Spi::new_rxonly(
        &mut spi_peri,
        &mut sck,
        &mut miso,
        // SPIv1/f1 requires txdma even if rxonly.
        #[cfg(not(any(
            feature = "stm32h503rb",
            feature = "stm32h563zi",
            feature = "stm32h753zi",
            feature = "stm32h755zi",
            feature = "stm32h7a3zi",
            feature = "stm32h7s3l8",
            feature = "stm32u585ai",
            feature = "stm32u5a5zj",
            feature = "stm32wba52cg",
        )))]
        &mut tx_dma,
        &mut rx_dma,
        spi_config,
    );

    let mut mosi = Output::new(&mut mosi, Level::Low, Speed::VeryHigh);

    mosi.set_high();
    spi.read(&mut buf).await.unwrap();
    assert_eq!(buf, [0xff; 9]);

    mosi.set_low();
    spi.read(&mut buf).await.unwrap();
    assert_eq!(buf, [0x00; 9]);

    info!("Test OK");
    cortex_m::asm::bkpt();
}
